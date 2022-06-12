use std::cmp;
use aector::actor::{Actor, MailboxType};
use aector::actor_system::ActorSystem;
use aector::Addr;
use aector::behavior::{Behavior, BehaviorAction, BehaviorBuilder};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::rect::{Point, Rect};
use sdl2::render::WindowCanvas;
use std::sync::{LockResult};
use sdl2::EventPump;

use crate::person::PopulationType;
use crate::grid::GridT;
use crate::protocol::*;
use crate::N;

pub struct UIGridData {
    grid: Arc<Mutex<Option<Box<GridT>>>>
}

impl UIGridData {
    pub fn new(grid_store: Arc<Mutex<Option<Box<GridT>>>>) -> Actor<Self> {

        // gets snapshot of grid at each end of iteration
        let b = BehaviorBuilder::new()
            .on_tell::<GridSnapshot>(|msg, state, ctx| -> BehaviorAction<UIGridData> {
                let l = state.grid.lock();
                match l {
                    Ok(mut grid) => {
                        *grid = Some(msg.grid);
                    },
                    Err(_) => {
                        println!("err acquiring lock of grid state");
                    }
                }
                Behavior::keep()
            })
            .build();

        let data = UIGridData {
            grid: grid_store
        };

        Actor::new(data, b, MailboxType::Unbounded)
    }
}

type SharedData = Arc<Mutex<Option<Box<GridT>>>>;

pub struct UI {
    canvas: WindowCanvas,
    event_pump: EventPump,
    data: SharedData,
    window_size: u32,
    fields: u32,
    spacing: i32
}

impl UI {
    pub fn new(data: SharedData, window_size: u32, fields: u32) -> Self {
        let sdl_context = sdl2::init().expect("sdl init failed");
        let video_subsystem = sdl_context.video().expect("video init failed");

        let window = video_subsystem
            .window("ABS Schelling Segregation", window_size, window_size)
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string()).expect("window creation error");

        let mut canvas = window.into_canvas().build().map_err(|e| e.to_string()).expect("canvas error");

        let mut event_pump = sdl_context.event_pump().expect("failed to create event pump");

        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();

        let spacing = (window_size / fields) as i32;

        UI {
            canvas,
            event_pump,
            data,
            window_size,
            fields,
            spacing
        }
    }

    pub fn run(&mut self) {
        'running: loop {
            for event in self.event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }
            self.canvas.present();
            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 30));
            self.canvas.set_draw_color(Color::RGB(255, 255, 255));
            self.canvas.clear();

            self.draw_grid();

            let l = self.data.lock();
            match l {
                Ok(grid) => {
                    match &*grid {
                        None => {
                            // no grid snapshot available yet
                        },
                        Some(grid) => {
                            self.draw_grid_content(&grid);
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }

    fn draw_grid(&mut self) {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));

        // draw grid
        for y in 1..(self.fields as i32) {
            for x in 1..(self.fields as i32) {
                self.canvas.draw_line(Point::new(0, y * self.spacing), Point::new(self.window_size as i32, y * self.spacing));
                self.canvas.draw_line(Point::new(x * self.spacing, 0), Point::new(x * self.spacing, self.window_size as i32));
            }
        }
    }

    fn draw_grid_content(&self, grid: &GridT) {
        for y in 0..N {
            for x in 0..N {
                match grid[y][x] {
                    None => {},
                    Some(race) => {
                        let field_center_x = ((x as i32) * self.spacing + (self.spacing / 2)) as i16;
                        let field_center_y = ((y as i32) * self.spacing + (self.spacing / 2)) as i16;
                        // circle diameter = 80% of field size, radius = 0.5 * diameter
                        let radius = ((self.spacing as f32) * 0.8 * 0.5) as i16;
                        match race {
                            PopulationType::A => {
                                self.canvas.filled_circle(field_center_x, field_center_y, radius, Color::RGB(255, 0, 0));
                            },
                            PopulationType::B => {
                                self.canvas.filled_circle(field_center_x, field_center_y, radius, Color::RGB(0, 255, 0));
                            }
                        }
                    }
                }
            }
        }
    }
}
