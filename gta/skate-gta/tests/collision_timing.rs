#![cfg(feature = "own-core")]
//! How long Skate takes to build collision of a scanned street's size
//! (run with --ignored --nocapture, in release).

#[test]
#[ignore]
fn collision_build_time() {
    let builder = skate_host::bridge::CollisionBuilder::for_timing();
    let mut tris = Vec::new();
    for i in 0..90 {
        for j in 0..70 {
            let (x, z) = (i as f32 * 0.2, j as f32 * 0.2);
            let y = ((i * 7 + j * 3) % 5) as f32 * 0.01;
            tris.push([[x, y, z], [x, y, z + 0.2], [x + 0.2, y, z + 0.2]]);
            tris.push([[x, y, z], [x + 0.2, y, z + 0.2], [x + 0.2, y, z]]);
        }
    }
    let n = tris.len();
    let t = std::time::Instant::now();
    for _ in 0..5 {
        builder.build_with_surfaces(tris.clone(), None, vec![]).unwrap();
    }
    eprintln!("{n} triangles: {:?} per build", t.elapsed() / 5);
}
