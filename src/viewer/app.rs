use std::{
    any::Any, cell::Cell, sync::{Arc, atomic::{AtomicBool, AtomicU64}, mpsc::{
        Receiver, RecvError, Sender, channel
    }}, thread::{JoinHandle, Thread}, time::Duration
};


use crossterm::{event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent}, terminal::{EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{
    DefaultTerminal, Frame, layout::Size
};

use crate::viewer::widgets::CursorBox;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Receive Error: {0}")]
    Recv(#[from] RecvError),
}

pub type Result<T = (), E = AppError> = std::result::Result<T, E>;

pub struct AppRequests {
    exit: Cell<bool>,
}

impl AppRequests {
    pub fn new() -> Self {
        Self {
            exit: Cell::new(false),
        }
    }

    pub fn request_exit(&self) {
        self.exit.set(true);
    }

    pub fn exit_requested(&self) -> bool {
        self.exit.get()
    }
}

pub struct AppContext {
    focused: bool,
    size: Size,
    last_mouse_pos: Option<(u16, u16)>,
}

impl AppContext {
    pub fn new() -> Self {
        Self {
            focused: false,
            size: Size::new(0, 0),
            last_mouse_pos: None,
        }
    }
}

pub enum EventSource {
    Crossterm,
    
}

pub enum AppEvent {
    Term(Event),
    IoError(EventSource, Box<std::io::Error>),
    Responder(Box<dyn FnOnce(&mut QueryApp) -> Result + Send>),
}

pub struct CrosstermThread {
    handle: JoinHandle<()>,
}

impl CrosstermThread {
    fn thread(stopper: Arc<AtomicBool>, sender: Sender<AppEvent>) {
        const SEND_FAILURE: &'static str = "Failed to send event";
        const WAIT: Duration = Duration::from_millis(100);
        loop {
            if stopper.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            match crossterm::event::poll(WAIT) {
                Ok(true) => {
                    let event = crossterm::event::read().unwrap();
                    sender.send(AppEvent::Term(event)).expect(SEND_FAILURE);
                }
                Ok(false) => {}
                Err(err) => {
                    sender.send(AppEvent::IoError(EventSource::Crossterm, Box::new(err))).expect(SEND_FAILURE);
                }
            }
        }
    }
    
    pub fn start(stopper: Arc<AtomicBool>, sender: Sender<AppEvent>) -> Self {
        Self {
            handle: std::thread::spawn(move || CrosstermThread::thread(stopper, sender)),
        }
    }
}

pub struct QueryApp {
    requests: AppRequests,
    ctx: AppContext,
    crossterm_thread: CrosstermThread,
    background_stopper: Arc<AtomicBool>,
    event_queue: Receiver<AppEvent>,
    event_sender: Sender<AppEvent>,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(u64);

impl EventId {
    const UNIQUE_START: u64 = 2u64.pow(32);
    #[must_use]
    #[inline(always)]
    pub fn unique() -> Self {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(EventId::UNIQUE_START);
        let next_id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(next_id)
    }

    #[must_use]
    #[inline(always)]
    pub const fn eq(self, other: Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    #[inline(always)]
    pub const fn ne(self, other: Self) -> bool {
        self.0 != other.0
    }
}

impl QueryApp {
    pub fn new() -> Self {
        let stopper = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = channel();
        Self {
            requests: AppRequests::new(),
            ctx: AppContext::new(),
            crossterm_thread: CrosstermThread::start(stopper.clone(), sender.clone()),
            event_queue: receiver,
            event_sender: sender,
            background_stopper: stopper,
        }
    }

    pub fn create_and_run() -> Result<(), AppError> {
        let mut app = Self::new();
        app.run()
    }

    fn handle_paste(&mut self, pasted: String) -> Result {
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result {
        match key.code {
            KeyCode::Esc => {
                self.requests.request_exit();
            }
            _ => (),
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result {
        self.ctx.last_mouse_pos = Some((mouse.column, mouse.row));
        Ok(())
    }

    fn handle_event(&mut self, event: AppEvent) -> Result {
        match event {
            AppEvent::Term(event) => {
                match event {
                    Event::FocusGained => self.ctx.focused = true,
                    Event::FocusLost => self.ctx.focused = false,
                    Event::Key(key_event) => self.handle_key(key_event)?,
                    Event::Mouse(mouse_event) => self.handle_mouse(mouse_event)?,
                    Event::Paste(pasted) => self.handle_paste(pasted)?,
                    Event::Resize(width, height) => self.ctx.size = Size::new(width, height),
                }
            },
            AppEvent::IoError(_, error) => {
                return Err(AppError::Io(*error));
            },
            AppEvent::Responder(resp) => {
                resp(self)?;
            },
        }
        Ok(())
    }

    fn draw(&mut self, term: &mut DefaultTerminal) -> Result {
        term.draw(move |frame| {
            
            if let Some((col, row)) = self.ctx.last_mouse_pos {
                frame.render_widget(CursorBox(col, row), frame.area());
            }
        })?;
        Ok(())
    }

    pub fn run(&mut self) -> Result {
        let mut terminal = ratatui::init();
        crossterm::execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
        )?;
        loop {
            let next_event = self.event_queue.recv()?;
            self.handle_event(next_event)?;
            if self.requests.exit_requested() {
                self.background_stopper.store(true, std::sync::atomic::Ordering::Relaxed);
                break;
            }
            self.draw(&mut terminal)?;
        }
        crossterm::execute!(
            terminal.backend_mut(),
            DisableBracketedPaste,
            DisableMouseCapture,
            LeaveAlternateScreen,
        )?;
        ratatui::restore();
        Ok(())
    }
}
