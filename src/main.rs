use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use rusty_audio::Audio;
use std::{
    error::Error,
    sync::mpsc::{self},
    time::{Duration, Instant},
    {io, thread},
};

use invaders::{
    frame::{self, new_frame, Drawable},
    invaders::Invaders,
    player::Player,
    render,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = Audio::new();
    audio.add("explode", "./sounds/explode.wav");
    audio.add("lose", "./sounds/lose.wav");
    audio.add("move", "./sounds/move.wav");
    audio.add("startup", "./sounds/startup.wav");
    audio.add("win", "./sounds/win.wav");
    audio.add("pew", "./sounds/pew.wav");
    audio.play("startup");

    //Terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;

    //Render loop in a separate thread
    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = thread::spawn(move || {
        let mut last_frame = frame::new_frame();
        let mut stdout = io::stdout();
        render::render(&mut stdout, &last_frame, &last_frame, true);
        loop {
            let curr_frame = match render_rx.recv() {
                Ok(x) => x,
                Err(_) => break,
            };
            render::render(&mut stdout, &last_frame, &curr_frame, false);
            last_frame = curr_frame;
        }
    });

    //Game loop
    let mut player = Player::new();
    let mut instant = Instant::now();
    let mut invaders = Invaders::new();

    'gameloop: loop {
        // Per-frame init
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut curr_frame = new_frame();
        //Input
        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Left => player.move_left(),
                    KeyCode::Right => player.move_right(),
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        if player.shoot() {
                            audio.play("pew");
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("lose");
                        break 'gameloop;
                    }
                    _ => {}
                }
            }
        }
        // Updates
        player.update(delta);
        if invaders.update(delta) {
            audio.play("move");
        }
        let hits: u16 = player.detect_hits(&mut invaders);
        if hits > 0 {
            audio.play("explode");
        }

        //Draw & render
        let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];
        for drawable in drawables {
            drawable.draw(&mut curr_frame);
        }
        let _ = render_tx.send(curr_frame);
        thread::sleep(Duration::from_millis(1));

        // Win or lose?
        if invaders.all_killed() {
            audio.play("win");
            break 'gameloop;
        } else if invaders.reached_bottom() {
            audio.play("lose");
            break 'gameloop;
        }
    }

    //Cleanup
    drop(render_tx);
    render_handle.join().unwrap();
    audio.wait();
    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
