//! The Skate engine on its own thread, as the mashup runs it: one session,
//! loaded once (it takes seconds) and kept while skate mode is off.
//! Collision is built on a second thread and swapped in between ticks.
//!
//! A spare session (about 40 MB) loads in the background. When Skate's
//! physics fails (a non-finite value: the session cannot go on), the spare
//! takes over where the skater was, with the same collision and options,
//! instead of skate mode stopping for an 8 s reload.

use bevy::math::{Mat3, Mat4, Vec3};
use skate_host::bridge::{InputFrame, PreparedCollision, Session, Surface};
use std::path::PathBuf;
use std::sync::mpsc;

/// Skate 3's options (see `config`).
#[derive(Clone, Debug)]
pub struct Options {
    pub difficulty: String,
    pub trucks: f32,
    pub wheels: f32,
    pub tuning: [f32; 11],
    pub hold_fakie: bool,
    pub hippie_jump: f32,
}

/// One published Skate pose, in Skate space.
#[derive(Clone, Debug)]
pub struct Frame {
    pub root: Mat4,
    pub bones: Vec<Mat4>,
    /// Position, basis (columns right, up, forward) and field of view.
    pub camera: Option<(Vec3, Mat3, f32)>,
    pub board: Mat4,
    pub velocity: Vec3,
    pub state: String,
    pub tick: u64,
    /// Skate 3's trick recognition and scoring.
    pub score: skate_host::ScoreView,
    /// The session marker, for its prompt.
    pub marker: skate_host::MarkerView,
    /// Skate 3's ground surface under the wheels (1 to 5) and the wheels
    /// down, for the board's sounds and wear.
    pub surface: u8,
    pub wheels_down: u8,
}

pub enum Event {
    /// The session is loaded; its poses carry these bones, in order.
    Ready { names: Vec<String> },
    Activated(Frame),
    Frame(Frame),
    /// The session stopped; skate mode must restart it.
    Failed(String),
    Note(String),
}

enum Job {
    Options(Options),
    /// Skate-space collision; `now` installs it before the next job.
    Collision { triangles: Vec<[[f32; 3]; 3]>, surfaces: Option<Vec<Surface>>, rails: Vec<Vec<[f32; 3]>>, now: bool },
    Activate { spawn: [f32; 3], heading: f32, on_foot: bool, speed: f32 },
    Step { dt: f32, aspect_ratio: f32, pad: Option<skate_input::Pad> },
    Suspend,
    /// Back to the session marker, or the checkpoint before the last trick.
    Retry,
}

pub struct Worker {
    send: mpsc::Sender<Job>,
    receive: mpsc::Receiver<Event>,
}

/// A small floor to load the session on before the world is scanned.
fn floor() -> Vec<[[f32; 3]; 3]> {
    let (a, b, c, d) = ([-2., 0., -2.], [2., 0., -2.], [2., 0., 2.], [-2., 0., 2.]);
    // Counterclockwise seen from above (Skate's +Y).
    vec![[a, d, c], [a, c, b]]
}

fn frame(pose: skate_host::bridge::Pose) -> Frame {
    Frame {
        root: pose.root,
        bones: pose.bones,
        camera: pose.camera,
        board: pose.board,
        velocity: pose.velocity,
        state: pose.state,
        tick: pose.tick,
        score: pose.score,
        marker: pose.marker,
        surface: pose.surface,
        wheels_down: pose.wheels_down,
    }
}

impl Worker {
    /// Starts loading the Skate session from the converted Skate 3 data.
    pub fn start(assets: PathBuf) -> Result<Self, String> {
        let (send, jobs) = mpsc::channel::<Job>();
        let (publish, receive) = mpsc::channel::<Event>();
        std::thread::Builder::new()
            .name("skate-gta".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                if let Err(e) = run(&assets, &jobs, &publish) {
                    let _ = publish.send(Event::Failed(e));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { send, receive })
    }

    fn post(&self, job: Job) {
        let _ = self.send.send(job);
    }

    pub fn options(&self, options: Options) {
        self.post(Job::Options(options));
    }

    /// New collision; `surfaces` gives Skate's ground (1 to 5) of each
    /// triangle, or none for all smooth.
    pub fn collision(&self, triangles: Vec<[[f32; 3]; 3]>, surfaces: Option<Vec<u8>>, rails: Vec<Vec<[f32; 3]>>, now: bool) {
        let surfaces = surfaces.map(|s| s.into_iter().map(Surface::from_index).collect());
        self.post(Job::Collision { triangles, surfaces, rails, now });
    }

    /// Starts skating at `spawn`, on foot with the board in hand or on it;
    /// on it rolling ahead at `speed` m/s (getting on at a run).
    pub fn activate(&self, spawn: [f32; 3], heading: f32, on_foot: bool, speed: f32) {
        self.post(Job::Activate { spawn, heading, on_foot, speed });
    }

    /// Advances by `dt` seconds with the controller and keys in `pad`, or
    /// no input.
    pub fn step(&self, dt: f32, aspect_ratio: f32, pad: Option<skate_input::Pad>) {
        self.post(Job::Step { dt, aspect_ratio, pad });
    }

    pub fn suspend(&self) {
        self.post(Job::Suspend);
    }

    /// Back to the session marker, or without one to where the skater last
    /// rolled calmly before their last trick (a failed trick or line).
    pub fn retry(&self) {
        self.post(Job::Retry);
    }

    pub fn events(&self) -> Vec<Event> {
        let mut events = Vec::new();
        loop {
            match self.receive.try_recv() {
                Ok(event) => events.push(event),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    events.push(Event::Failed("the Skate thread ended".into()));
                    break;
                }
            }
        }
        events
    }
}

/// Loads a spare session on its own thread.
fn load_spare(assets: &std::path::Path) -> mpsc::Receiver<Result<Session, String>> {
    let (send, receive) = mpsc::channel();
    let assets = assets.to_path_buf();
    let _ = std::thread::Builder::new()
        .name("skate-gta-spare".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let _ = send.send(Session::new(&assets, floor(), vec![], [0., 0., 0.], 0.));
        });
    receive
}

/// Where the skater was at the last good frame: the deck's place (Skate
/// space), the heading, and whether on foot.
#[derive(Clone, Copy)]
struct Place {
    at: [f32; 3],
    heading: f32,
    on_foot: bool,
}

fn place_of(pose: &skate_host::bridge::Pose, on_foot: bool) -> Place {
    let forward = pose.root.z_axis;
    Place { at: pose.board.w_axis.truncate().to_array(), heading: forward.x.atan2(forward.z), on_foot }
}

type Build = (Vec<[[f32; 3]; 3]>, Option<Vec<Surface>>, Vec<Vec<[f32; 3]>>);

/// The spare in place of a failed session: options, the newest collision,
/// the skater where they were. The failed session is dropped.
fn take_over(
    spare: Session,
    options: Option<&Options>,
    collision: Option<&Build>,
    place: Option<Place>,
) -> Result<(Session, Option<skate_host::bridge::Pose>), String> {
    let mut session = spare;
    if let Some(o) = options {
        session.apply_options(&o.difficulty, o.trucks, o.wheels, o.tuning, o.hold_fakie)?;
        session.set_hippie_jump(o.hippie_jump)?;
    }
    if let Some((triangles, surfaces, rails)) = collision {
        let prepared = session.collision_builder().build_with_surfaces(triangles.clone(), surfaces.clone(), rails.clone())?;
        session.install_collision(prepared)?;
    }
    let pose = match place {
        Some(p) if p.on_foot => Some(session.activate_on_foot(p.at, p.heading)?),
        Some(p) => Some(session.activate(p.at, p.heading)?),
        None => None,
    };
    Ok((session, pose))
}

fn run(assets: &std::path::Path, jobs: &mpsc::Receiver<Job>, publish: &mpsc::Sender<Event>) -> Result<(), String> {
    let started = std::time::Instant::now();
    let mut session = Session::new(assets, floor(), vec![], [0., 0., 0.], 0.)?;
    let mut spare_loading = Some(load_spare(assets));
    let mut spare: Option<Session> = None;
    let mut options: Option<Options> = None;
    let mut collision: Option<Build> = None;
    let mut place: Option<Place> = None;
    let mut active = false;
    let names = session.pose().names;
    let _ = publish.send(Event::Note(format!(
        "Skate session loaded in {} ms, {} bones",
        started.elapsed().as_millis(),
        names.len()
    )));
    if publish.send(Event::Ready { names }).is_err() {
        return Ok(());
    }

    // Collision for later swaps is built off this thread.
    let builder = session.collision_builder();
    let (build, builds) = mpsc::channel::<Build>();
    let (built_send, built) = mpsc::channel::<Result<PreparedCollision, String>>();
    std::thread::Builder::new()
        .name("skate-gta-collision".into())
        .spawn(move || {
            while let Ok(mut job) = builds.recv() {
                // Only the newest scan matters.
                while let Ok(newer) = builds.try_recv() {
                    job = newer;
                }
                if built_send.send(builder.build_with_surfaces(job.0, job.1, job.2)).is_err() {
                    break;
                }
            }
        })
        .map_err(|e| e.to_string())?;

    let mut packet = 0u32;
    let mut last_input;
    let mut accumulated = 0f32;
    while let Ok(job) = jobs.recv() {
        if spare.is_none()
            && let Some(loading) = spare_loading.as_ref()
        {
            match loading.try_recv() {
                Ok(Ok(loaded)) => {
                    spare = Some(loaded);
                    spare_loading = None;
                }
                Ok(Err(e)) => {
                    let _ = publish.send(Event::Note(format!("Skate spare session: {e}")));
                    spare_loading = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => spare_loading = None,
            }
        }
        match job {
            Job::Options(o) => {
                options = Some(o.clone());
                if let Err(e) = session
                    .apply_options(&o.difficulty, o.trucks, o.wheels, o.tuning, o.hold_fakie)
                    .and_then(|()| session.set_hippie_jump(o.hippie_jump))
                {
                    let _ = publish.send(Event::Note(format!("Skate options: {e}")));
                }
            }
            Job::Collision { triangles, surfaces, rails, now: true } => {
                collision = Some((triangles.clone(), surfaces.clone(), rails.clone()));
                let prepared = session.collision_builder().build_with_surfaces(triangles, surfaces, rails)?;
                session.install_collision(prepared)?;
            }
            Job::Collision { triangles, surfaces, rails, now: false } => {
                collision = Some((triangles.clone(), surfaces.clone(), rails.clone()));
                let _ = build.send((triangles, surfaces, rails));
            }
            Job::Activate { spawn, heading, on_foot, speed } => {
                accumulated = 0.;
                let pose = if on_foot {
                    session.activate_on_foot(spawn, heading)?
                } else {
                    let pose = session.activate(spawn, heading)?;
                    if speed > 0. {
                        // Skate's forward is +Z turned by the heading about Y.
                        session.roll([heading.sin() * speed, 0., heading.cos() * speed]);
                    }
                    pose
                };
                active = true;
                place = Some(place_of(&pose, session.on_foot()));
                if publish.send(Event::Activated(frame(pose))).is_err() {
                    return Ok(());
                }
            }
            Job::Suspend => {
                accumulated = 0.;
                session.suspend_input();
            }
            Job::Retry => session.return_to_checkpoint(),
            Job::Step { dt, aspect_ratio, pad } => {
                while let Ok(prepared) = built.try_recv() {
                    match prepared {
                        Ok(prepared) => session.install_collision(prepared)?,
                        Err(e) => {
                            let _ = publish.send(Event::Note(format!("Skate collision: {e}")));
                        }
                    }
                }
                session.set_aspect_ratio(aspect_ratio);
                packet = packet.wrapping_add(1);
                let input = match pad {
                    Some(pad) => {
                        let (left, right) = pad.axes();
                        InputFrame::from_pad(pad.buttons, pad.triggers, left, right, packet)
                    }
                    None => InputFrame::neutral(),
                };
                last_input = format!("buttons {:#06x}, left {:?}, right {:?}", input.buttons(), pad.map(|p| p.left), pad.map(|p| p.right));
                session.collect(input, dt);
                accumulated = (accumulated + dt).min(0.15);
                let mut advanced = false;
                let mut failed = None;
                while accumulated >= session.period() {
                    accumulated -= session.period();
                    // A failure says what went in last: the next one can be found.
                    if let Err(e) = session.advance() {
                        failed = Some(format!("{e} (last input: {last_input}; on foot: {})", session.on_foot()));
                        break;
                    }
                    advanced = true;
                }
                let mut pose = None;
                if failed.is_none() && advanced {
                    let p = session.pose();
                    if !p.root.is_finite() || p.bones.iter().any(|b| !b.is_finite()) {
                        failed = Some("Skate published a non-finite pose".into());
                    } else {
                        pose = Some(p);
                    }
                }
                if let Some(e) = failed {
                    // The spare goes on where the skater was; without one
                    // (still loading, or it failed too) skate mode stops.
                    let Some(fresh) = spare.take() else { return Err(e) };
                    let recovered = std::time::Instant::now();
                    let (taken, activated) = take_over(fresh, options.as_ref(), collision.as_ref(), place.filter(|_| active))
                        .map_err(|e2| format!("{e}; the spare session failed too: {e2}"))?;
                    session = taken;
                    accumulated = 0.;
                    spare_loading = Some(load_spare(assets));
                    let _ = publish.send(Event::Note(format!(
                        "Skate failed and went on in its spare session ({} ms): {e}",
                        recovered.elapsed().as_millis()
                    )));
                    if let Some(p) = activated
                        && publish.send(Event::Activated(frame(p))).is_err()
                    {
                        return Ok(());
                    }
                    continue;
                }
                if let Some(pose) = pose {
                    place = Some(place_of(&pose, session.on_foot()));
                    if publish.send(Event::Frame(frame(pose))).is_err() {
                        return Ok(());
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod spare_tests {
    use super::*;

    /// A flat 200 m square, counterclockwise from above.
    fn ground() -> Vec<[[f32; 3]; 3]> {
        let (a, b, c, d) = ([-100., 0., -100.], [100., 0., -100.], [100., 0., 100.], [-100., 0., 100.]);
        vec![[a, d, c], [a, c, b]]
    }

    #[test]
    #[ignore = "reads the real Skate data (about 15 s)"]
    fn getting_on_at_a_run_rolls_on() {
        let assets = std::path::Path::new(r"C:\Games\2010-Rust-Rewrite-Mashup\skate-data\assets");
        let mut session = Session::new(assets, floor(), vec![], [0., 0., 0.], 0.).unwrap();
        let prepared = session.collision_builder().build_with_surfaces(ground(), None, vec![]).unwrap();
        session.install_collision(prepared).unwrap();
        let heading = 1.0f32;
        session.activate([3., 0.1, 5.], heading).unwrap();
        session.roll([heading.sin() * 6., 0., heading.cos() * 6.]);
        let start = session.pose().board.w_axis.truncate();
        for _ in 0..60 {
            session.collect(InputFrame::neutral(), 1. / 60.);
            session.advance().unwrap();
        }
        let pose = session.pose();
        let moved = pose.board.w_axis.truncate() - start;
        let ahead = Vec3::new(heading.sin(), 0., heading.cos());
        eprintln!("1 s later: moved {moved}, state {}, speed {}", pose.state, pose.velocity.length());
        assert!(moved.dot(ahead) > 3., "rolled ahead: {moved}");
        assert!(!pose.state.contains("Wipeout"), "no bail: {}", pose.state);
    }

    #[test]
    #[ignore = "reads the real Skate data (about 20 s)"]
    fn the_spare_goes_on_where_the_skater_was() {
        let assets = std::path::Path::new(r"C:\Games\2010-Rust-Rewrite-Mashup\skate-data\assets");
        let mut session = Session::new(assets, floor(), vec![], [0., 0., 0.], 0.).unwrap();
        let spare = load_spare(assets).recv().unwrap().unwrap();
        let build = (ground(), None, vec![]);
        let prepared = session.collision_builder().build_with_surfaces(build.0.clone(), None, vec![]).unwrap();
        session.install_collision(prepared).unwrap();
        session.activate([3., 0.1, 5.], 1.0).unwrap();
        for _ in 0..120 {
            session.collect(InputFrame::neutral(), 1. / 60.);
            session.advance().unwrap();
        }
        let before = session.pose();
        let place = place_of(&before, session.on_foot());
        let options = Options { difficulty: "normal".into(), trucks: 0.7, wheels: 0.7, tuning: [1.; 11], hold_fakie: false, hippie_jump: 1.6 };
        let (mut taken, pose) = take_over(spare, Some(&options), Some(&build), Some(place)).unwrap();
        let pose = pose.unwrap();
        let (was, now) = (before.board.w_axis.truncate(), pose.board.w_axis.truncate());
        eprintln!("before {was} heading {}, after {now}", place.heading);
        assert!(was.distance(now) < 1.0, "on where it was: {was} -> {now}");
        for _ in 0..120 {
            taken.collect(InputFrame::neutral(), 1. / 60.);
            taken.advance().unwrap();
        }
        let after = taken.pose();
        assert!(after.root.is_finite() && after.bones.iter().all(|b| b.is_finite()));
        assert!(after.board.w_axis.y > -0.5, "still on the ground: {}", after.board.w_axis);
    }
}
