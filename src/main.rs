use std::{
    path::Path,
    time::Duration,
    error::Error,
    panic::{self, PanicInfo},
    io,
};
use env_logger::{Builder, Env};
use log::info;

#[macro_use]
extern crate lazy_static;

use backtrace::Backtrace;

use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Terminal,
};

mod driver;
use driver::*;

mod aqi;
use aqi::aqi_from_pm2_5;

mod event;
use event::*;


struct App {
    sensor: Sensor,
    pm_2_5_data: Vec<(f64, f64)>,
    window: [f64; 2],
}

impl App {
    fn new(sensor: Sensor) -> App {
        App {
            sensor,
            pm_2_5_data: vec![],
            window: [0.0, 20.0],
        }
    }

    fn update(&mut self) {
        let measurement = self.sensor.get_measurement()
            .expect("Failed to get measurement");
        let aqi = aqi_from_pm2_5(measurement.pm2_5);

        if self.pm_2_5_data.len() > 20 {
            self.pm_2_5_data.remove(0);
            self.window[0] += 1.0;
            self.window[1] += 1.0;
            self.pm_2_5_data.push((self.window[1], aqi as f64));
        } else {
            self.pm_2_5_data.push((self.pm_2_5_data.len() as f64, aqi as f64));
        }
    }
}


fn run() -> Result<(), Box<dyn Error>> {
    let path = Path::new("/dev/tty.usbserial-14110");
    let mut sensor = Sensor::new(path)
        .expect("Unable to open device");
    info!("Opened device at path: {:?}", path);

    sensor.configure(Duration::from_secs(1))
        .expect("Failed to configure device");
    info!("Configured device");

    // Wake the sensor in case it's sleepinng
    let wake_command = SendData::set_work_state(WorkState::Measuring);
    sensor.send(&wake_command).expect("Failed to send wake command");
    // Set the report mode
    sensor.send(&SendData::set_report_mode(ReportMode::Initiative))
        .expect("Failed to set report mode to initiative");

    let mut app = App::new(sensor);

    // Initialize the terminal
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup event handlers
    let config = Config {
        tick_rate: Duration::from_secs(1),
        ..Default::default()
    };
    let events = Events::with_config(config);

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(10),
                        Constraint::Percentage(40),
                        Constraint::Percentage(40),
                        Constraint::Percentage(10),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let x_labels = vec![
                Span::styled(
                    format!("{}", app.window[0]),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", (app.window[0] + app.window[1]) / 2.0)),
                Span::styled(
                    format!("{}", app.window[1]),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ];

            let datasets = vec![Dataset::default()
                                .name("data")
                                .marker(symbols::Marker::Braille)
                                .style(Style::default().fg(Color::Yellow))
                                .graph_type(GraphType::Line)
                                .data(&app.pm_2_5_data)];

            let chart = Chart::new(datasets)
                .block(
                    Block::default()
                        .title(Span::styled(
                            "Air Quality Index (PM 2.5)",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .borders(Borders::ALL),
                )
                .x_axis(
                    Axis::default()
                        .title("X Axis")
                        .style(Style::default().fg(Color::Gray))
                        .bounds(app.window)
                        .labels(x_labels),
                )
                .y_axis(
                    Axis::default()
                        .title("Y Axis")
                        .style(Style::default().fg(Color::Gray))
                        .bounds([0.0, 500.0])
                        .labels(vec![
                            Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                            Span::raw("2.5"),
                            Span::styled("5.0", Style::default().add_modifier(Modifier::BOLD)),
                            Span::styled("500.0", Style::default().add_modifier(Modifier::BOLD)),
                        ]),
                );
            f.render_widget(chart, chunks[1]);

        })?;

        match events.next()? {
            Event::Tick => app.update(),
            Event::Input(Key::Char('q')) => {
                break;
            },
            Event::Input(_) => (),
        };
    }

    Ok(())
}

/// Shows a backtrace if the program panics
fn panic_hook(info: &PanicInfo<'_>) {
    if cfg!(debug_assertions) {
        let location = info.location().unwrap();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        let stacktrace: String = format!("{:?}", Backtrace::new()).replace('\n', "\n\r");

        println!(
            "{}thread '<unnamed>' panicked at '{}', {}\n\r{}",
            termion::screen::ToMainScreen,
            msg,
            location,
            stacktrace
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    panic::set_hook(Box::new(|info| {
        panic_hook(info);
    }));

    // Set up logger environment
    Builder::from_env(Env::default().default_filter_or("trace"))
        .try_init()
        .unwrap_or_else(|err| eprintln!("env_logger::init() failed: {}", err));


    run()
}
