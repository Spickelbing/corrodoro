use crate::pomodoro::State;
use crate::protocol::{Event, NetworkProtocol, TimerVisuals};
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
use zwiesel::{Client, ClientError, ClientEvent, Server, ServerError, ServerEvent};

pub struct App {
    pomodoro_state: State,
    tui: Tui,
    server: Option<Server<NetworkProtocol>>,
}

impl App {
    pub fn new(pomodoro_state: State) -> Result<Self, UnrecoverableError> {
        let tui = Tui::new()?;

        Ok(Self {
            pomodoro_state,
            tui,
            server: None,
        })
    }

    pub async fn start_server(&mut self, socket: SocketAddr) -> Result<(), ServerError> {
        self.server = Some(Server::host(socket).await?);
        Ok(())
    }

    pub async fn stop_server(&mut self) -> () {
        if let Some(server) = &mut self.server {
            server.disconnect_all();
            self.server = None;
        }
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

        loop {
            let visuals = TimerVisuals::from(&*self);
            let network_status = NetworkStatus::from(&*self);
            self.tui.render(&visuals, &network_status)?;
            if let Some(server) = &mut self.server {
                let _ = server.broadcast(&NetworkProtocol::Display(visuals)).await?;
            }

            select! {
                _ = pomodoro_clock.tick() => {
                    if self.pomodoro_state.timer_is_active() {
                        let activity_before = self.pomodoro_state.current_activity();

                        self.pomodoro_state.increase_progress(pomodoro_start_time.elapsed());
                        pomodoro_start_time = Instant::now();

                        let activity_after = self.pomodoro_state.current_activity();
                        if activity_before != activity_after {
                            if let Some(server) = &mut self.server {
                                let _ = server.broadcast(&NetworkProtocol::Notify(activity_after)).await?;
                            }
                            self.tui.show_notification(&activity_after.to_string(), true);
                        }
                    }
                }
                tui_event = self.tui.read_event() => {
                    let event = tui_event?;
                    if *self.handle_event(&event, &mut pomodoro_clock, &mut pomodoro_start_time) {
                        break;
                    }
                }
                server_event = async {
                    match &mut self.server {
                        Some(server) => server.event().await,
                        _ => ForeverPending.await.forever(),
                    }
                } => {
                    if let Ok(event) = server_event {
                        match event {
                            ServerEvent::NewConnection(client_id) => {
                                self.tui.show_notification(&format!("Client {client_id} connected"), false);
                            }
                            ServerEvent::Disconnect(client_id, None) => {
                                self.tui.show_notification(&format!("Client {client_id} disconnected"), false);
                            }
                            ServerEvent::Disconnect(client_id, Some(_)) => {
                                self.tui.show_notification(&format!("Client {client_id} disconnected because of a network error"), false);
                            }
                            ServerEvent::Message(_, message) => {
                                match message {
                                    NetworkProtocol::Event(event) => {
                                        if *self.handle_event(&event, &mut pomodoro_clock, &mut pomodoro_start_time) {
                                            break;
                                        }
                                    }
                                    _ => (), // received wrong type of message, ignore for now
                                }
                            }
                        }
                    } else {
                        // ignore network errors for now
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
            Event::ResetTimer => {
                self.pomodoro_state.reset();
            }
            Event::Quit => return AppShouldQuit(true),
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
    client: Client<NetworkProtocol>,
}

impl ClientApp {
    pub async fn connect(addr: SocketAddr) -> Result<Self, UnrecoverableError> {
        let client = Client::connect(addr).await?;
        let tui = Tui::new()?;

        Ok(Self { tui, client })
    }

    pub async fn run(&mut self) -> Result<(), UnrecoverableError> {
        self.tui.enable()?;
        let result = self.run_inner().await;
        self.tui.disable()?;

        result
    }

    async fn run_inner(&mut self) -> Result<(), UnrecoverableError> {
        let network_status = NetworkStatus::from(&*self);

        loop {
            select! {
                event = self.tui.read_event() => {
                    let event = event?;
                    if let Event::Quit = event {
                        break;
                    }
                    self.client.send(&NetworkProtocol::Event(event)).await?;
                }
                event = self.client.event() => {
                    match event {
                        Ok(ClientEvent::Message(msg)) => {
                            match msg {
                                NetworkProtocol::Display(visuals) => self.tui.render(&visuals, &network_status)?,
                                NetworkProtocol::Notify(activity) => self.tui.show_notification(&activity.to_string(), true),
                                _ => (), // received wrong type of message, ignore for now
                            }
                        }
                        Ok(ClientEvent::Disconnect(None)) => {
                            return Err(ClientError::ServerDisconnect.into());
                        }
                        Ok(ClientEvent::Disconnect(Some(err))) => {
                            return Err(err.into());
                        }
                        Err(err) => {
                            return Err(err.into());
                        }
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

impl From<&App> for TimerVisuals {
    fn from(app: &App) -> Self {
        TimerVisuals {
            time_remaining: app.pomodoro_state.time_remaining(),
            timer_is_paused: !app.pomodoro_state.timer_is_active(),
            activity: app.pomodoro_state.current_activity(),
            progress_percentage: app.pomodoro_state.progress_percentage(),
            completed_focus_sessions: app.pomodoro_state.completed_focus_sessions(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetworkStatus {
    Offline,
    Server {
        connected_clients: Vec<String>,
        listening_on: SocketAddr,
    },
    Client {
        connected_to: SocketAddr,
    },
}

impl From<&App> for NetworkStatus {
    fn from(app: &App) -> Self {
        match &app.server {
            Some(server) => NetworkStatus::Server {
                connected_clients: server.clients().iter().map(|c| format!("{c}")).collect(),
                listening_on: server.local_addr,
            },
            None => NetworkStatus::Offline,
        }
    }
}

impl From<&ClientApp> for NetworkStatus {
    fn from(app: &ClientApp) -> Self {
        NetworkStatus::Client {
            connected_to: app.client.remote_addr,
        }
    }
}

/// Represents errors the app has no control over.
#[derive(Debug, Error)]
pub enum UnrecoverableError {
    #[error("error while interfacing with the terminal: {0}")]
    Tui(#[from] TuiError),
    #[error("network error: {0}")]
    NetworkClient(#[from] ClientError),
    #[error("network error: {0}")]
    NetworkServer(#[from] ServerError),
    #[error("failed to resolve hostname")]
    HostHasNoDnsRecords,
    #[error(
        "failed to resolve hostname to an ipv4 address (but it can be resolved to an ipv6 address)"
    )]
    HostHasOnlyIpv6Records,
    #[error(
        "failed to resolve hostname to an ipv6 address (but it can be resolved to an ipv4 address)"
    )]
    HostHasOnlyIpv4Records,
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
