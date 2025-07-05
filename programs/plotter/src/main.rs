use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use serialport::new;
use three_d::*;

#[derive(Debug, Clone, Copy)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

fn parse_quaternion(line: &str) -> Option<Quaternion> {
    if !line.contains("Quaternion") { return None; }

    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() != 4 { return None; }

    Some(Quaternion {
        w: parts[0].split('=').nth(1)?.trim().parse().ok()?,
        x: parts[1].split('=').nth(1)?.trim().parse().ok()?,
        y: parts[2].split('=').nth(1)?.trim().parse().ok()?,
        z: parts[3].split('=').nth(1)?.trim().parse().ok()?,
    })
}

fn read_quaternions(port_name: &str, shared_quat: Arc<Mutex<Quat>>) {
    let port = new(port_name, 115200)
        .timeout(std::time::Duration::from_millis(100))
        .open()
        .expect("Error abriendo el puerto serial");

    let reader = BufReader::new(port);

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(q) = parse_quaternion(&line) {
                let quat = Quat::new(q.w, q.x, q.y, q.z);
                let mut shared = shared_quat.lock().unwrap();
                *shared = quat;
            }
        }
    }
}

pub fn main() {
    let shared_quat = Arc::new(Mutex::new(Quat::new(1.0, 0.0, 0.0, 0.0)));
    let quat_clone = shared_quat.clone();

    thread::spawn(move || {
        read_quaternions("/dev/ttyACM0", quat_clone);
    });

    let window = Window::new(WindowSettings {
        title: "Rotating Cube with Quaternion".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();

    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(5.0, 2.0, 2.5),
        vec3(0.0, 0.0, -0.5),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        0.1,
        1000.0,
    );

    let mut control = OrbitControl::new(camera.target(), 1.0, 100.0);

    let mut cube = Gm::new(
        Mesh::new(&context, &CpuMesh::cube()),
        PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: Srgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 100,
                },
                ..Default::default()
            },
        ),
    );

    let axes = Axes::new(&context, 0.1, 2.0);

    let light0 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
    let light1 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));

    window.render_loop(move |mut frame_input| {
        camera.set_viewport(frame_input.viewport);
        control.handle_events(&mut camera, &mut frame_input.events);

        let quat = *shared_quat.lock().unwrap();
        let rotation = Mat4::from(quat);
        let transform = Mat4::from_translation(vec3(1.0, 1.0, 1.0)) * rotation * Mat4::from_scale(0.2);
        cube.set_transformation(transform);

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.2, 0.2, 0.2, 1.0, 1.0))
            .render(
                &camera,
                cube.into_iter().chain(&axes),
                &[&light0, &light1],
            );

        FrameOutput::default()
    });
}
