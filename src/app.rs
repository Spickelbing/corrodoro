use crate::app::net::{Message, NetworkError, RemoteServer, Server};
use crate::pomodoro;
use crate::tui::{Tui, TuiError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::ops::Deref;
use std::panic;
use std::task::Poll;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::select;
use tokio::time::{interval, Interval};

mod net;

pub struct App {
    pomodoro_state: pomodoro::State,
    tui: Tui,
    server: Option<Server>,
}

impl App {
    pub fn new(pomodoro_state: pomodoro::State) -> Result<Self, UnrecoverableError> {
        let tui = Tui::new()?;

        Ok(Self {
            pomodoro_state,
            tui,
            server: None,
        })
    }

    pub async fn start_server(&mut self, socket: SocketAddr) -> Result<(), NetworkError> {
        self.server = Some(Server::bind(socket).await?);
        Ok(())
    }

    pub async fn stop_server(&mut self) -> Result<(), Vec<NetworkError>> {
        if let Some(server) = &mut self.server {
            server.disconnect_all();
            self.server = None;
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), UnrecoverableError> {
        self.tui.enable()?;
        let maybe_err = self.run_inner().await;
        self.tui.disable()?;

        maybe_err?;
        Ok(())
    }

    async fn run_inner(&mut self) -> Result<(), UnrecoverableError> {
        let mut pomodoro_clock = interval(Duration::from_millis(100));
        let mut pomodoro_start_time = Instant::now();

        let mut notify_end_of_activity = false;
        loop {
            let ui_data = UiData::from(&*self);
            let ui_data = UiData {
                notify_end_of_activity,
                ..ui_data
            };

            self.tui.render(&ui_data)?;

            if let Some(server) = &mut self.server {
                server.accept_pending_connections().await;

                let ui_data = UiData {
                    mode_info: AppModeInfo::Client {
                        connected_to: server.local_addr,
                    },
                    ..ui_data
                };

                let message = Message::UiData(ui_data);
                // ignore transmission errors for now
                let _ = server.broadcast_message(message).await;
            }
            notify_end_of_activity = false;

            select! {
                _ = pomodoro_clock.tick() => {
                    if self.pomodoro_state.timer_is_active() {
                        let activity_before = self.pomodoro_state.current_activity();

                        self.pomodoro_state.increase_progress(pomodoro_start_time.elapsed());
                        pomodoro_start_time = Instant::now();

                        let activity_after = self.pomodoro_state.current_activity();
                        if activity_before != activity_after {
                            notify_end_of_activity = true;
                        }
                    }
                }
                event = self.tui.read_event() => {
                    let event = event?;
                    if *self.handle_event(&event, &mut pomodoro_clock, &mut pomodoro_start_time) {
                        break;
                    }
                }
                event = async {
                    match &mut self.server {
                        Some(server) => {
                            let msg = server.next_message().await?;
                            match msg {
                                Message::Event(event) => Ok(event),
                                _ => Err(NetworkError::UnexpectedMessage),
                            }
                        }
                        _ => ForeverPending.await.forever(),
                    }
                } => {
                    match event {
                        Ok(event) => {
                            if *self.handle_event(&event, &mut pomodoro_clock, &mut pomodoro_start_time) {
                                break;
                            }
                        }
                        Err(_err) => (), // network error occured, ignore for now
                    }
                }
            }
        }

        if let Some(server) = &mut self.server {
            server.disconnect_all();
        }

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: &Event,
        pomodoro_clock: &mut Interval,
        pomodoro_start_time: &mut Instant,
    ) -> AppShouldQuit {
        let timer_was_stopped = !self.pomodoro_state.timer_is_active();

        match event {
            Event::ToggleTimer => {
                self.pomodoro_state.toggle_timer();
            }
            Event::ExtendActivity(duration) => {
                self.pomodoro_state.extend_activity(duration);
            }
            Event::ReduceActivity(duration) => {
                self.pomodoro_state.reduce_activity(duration);
            }
            Event::SkipActivity => {
                self.pomodoro_state.skip_activity();
            }
            Event::Quit => return AppShouldQuit(true),
            _ => (),
        };

        let timer_is_active_now = self.pomodoro_state.timer_is_active();
        if timer_was_stopped && timer_is_active_now {
            pomodoro_clock.reset();
            *pomodoro_start_time = Instant::now();
        }

        AppShouldQuit(false)
    }
}

pub struct ClientApp {
    tui: Tui,
    remote_server: RemoteServer,
}

impl ClientApp {
    pub async fn connect(addr: SocketAddr) -> Result<Self, UnrecoverableError> {
        let remote_server = RemoteServer::connect(addr).await?;
        let tui = Tui::new()?;

        Ok(Self { tui, remote_server })
    }

    pub async fn run(&mut self) -> Result<(), UnrecoverableError> {
        self.tui.enable()?;
        let maybe_err = self.run_inner().await;
        self.tui.disable()?;

        maybe_err?;
        Ok(())
    }

    async fn run_inner(&mut self) -> Result<(), UnrecoverableError> {
        loop {
            // TODO: what about resize events?
            select! {
                event = self.tui.read_event() => {
                    let event = event?;

                    if let Event::Quit = event {
                        break;
                    }

                    let msg = Message::Event(event);
                    self.remote_server.send_message(msg).await?;
                }
                msg = self.remote_server.next_message() => {
                    let msg = msg?;
                    if let Message::UiData(render_data) = msg {
                        self.tui.render(&render_data)?;
                    } else {
                        // this should never happen
                        // TODO: handle error
                    }
                }
            }
        }

        Ok(())
    }
}

struct AppShouldQuit(bool);

impl Deref for AppShouldQuit {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// These are the events that can be sent to the app, either from a UI or via network.
#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    Quit,
    ToggleTimer,
    ResetTimer,
    SkipActivity,
    ExtendActivity(Duration),
    ReduceActivity(Duration),
}

/// This is the data that is sent to a UI for display.
#[derive(Serialize, Deserialize)]
pub struct UiData {
    pub time_remaining: pomodoro::SessionDuration,
    pub timer_is_paused: bool,
    pub activity: pomodoro::Activity,
    pub progress_percentage: f64,
    pub completed_focus_sessions: u32,
    pub notify_end_of_activity: bool,
    pub mode_info: AppModeInfo,
}

impl From<&App> for UiData {
    fn from(app: &App) -> Self {
        let mode_info = match &app.server {
            Some(server) => AppModeInfo::Server {
                connected_clients: server.clients(),
                listening_on: server.local_addr,
            },
            None => AppModeInfo::Offline,
        };

        UiData {
            time_remaining: app.pomodoro_state.time_remaining(),
            timer_is_paused: !app.pomodoro_state.timer_is_active(),
            activity: app.pomodoro_state.current_activity(),
            progress_percentage: app.pomodoro_state.progress_percentage(),
            completed_focus_sessions: app.pomodoro_state.completed_focus_sessions(),
            notify_end_of_activity: false,
            mode_info,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AppModeInfo {
    Offline,
    Server {
        connected_clients: Vec<SocketAddr>,
        listening_on: SocketAddr,
    },
    Client {
        connected_to: SocketAddr,
    },
}

/// Represents errors the app has no control over.
#[derive(Debug, Error)]
pub enum UnrecoverableError {
    #[error("error while interfacing with the terminal: {0}")]
    Tui(#[from] TuiError),
    #[error("network error: {0}")]
    Network(#[from] NetworkError),
}

struct ForeverPending;

impl ForeverPending {
    /// This function only exists to produce the never type.
    /// It is used to make the compiler happy when awaiting `ForeverPending` in a `select!` block as an alternative to another future,
    /// if that other future does not exist.
    /// This function should only be used on the result of an await on `ForeverPending`, as then it is never called.
    /// If `!` were stable, this function would not be necessary, because `ForeverPending` could implement `Future<Output = !>`.
    /// # Panics
    /// Always panics.
    fn forever(&self) -> ! {
        panic!("ForeverPending::forever() was called, which should never happen")
    }
}

impl std::future::Future for ForeverPending {
    type Output = ForeverPending;

    fn poll(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}
