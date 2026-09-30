//! Create an isolated repeatable scene for UI checks and resource measurements.
//! Run: cargo run --example scene -- /tmp/charlita-check 8
use charlita::{
    assets,
    model::{Character, Member, Person, Pose},
    store::Store,
};
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(args.len() == 3, "Usage: scene DATA_DIR GUEST_COUNT");
    let count: usize = args[2].parse()?;
    let store = Store::open(args[1].clone().into())?;
    let mut doc = store.load("live")?.unwrap();
    anyhow::ensure!(doc.people.is_empty(), "Use an empty data directory");
    doc.settings.auto_check = false;
    doc.settings.language = "es".into();
    doc.profiles[0].name = "Escena de prueba".into();
    doc.profiles[0].groups[0].height = 720;
    doc.profiles[0].groups[0].auto_size = false;
    doc.profiles[0].groups[0].layout = charlita::model::Layout::Grid;
    doc.profiles[0].groups[0].columns = 4;
    for i in 0..count {
        let color = [
            [126, 176, 230],
            [184, 160, 224],
            [222, 177, 135],
            [144, 194, 175],
        ][i % 4];
        let mut image = image::RgbaImage::new(128, 128);
        for y in 0i32..128 {
            for x in 0i32..128 {
                let head = (x - 64).pow(2) + (y - 48).pow(2) < 34 * 34;
                let body = (x - 64).pow(2) / 2 + (y - 117).pow(2) < 40 * 40;
                let eye = ((42..48).contains(&x) || (80..86).contains(&x)) && (42..53).contains(&y);
                let pixel = if eye {
                    [21, 24, 29, 255]
                } else if head || body {
                    [color[0], color[1], color[2], 255]
                } else {
                    [0, 0, 0, 0]
                };
                image.put_pixel(x as u32, y as u32, image::Rgba(pixel));
            }
        }
        let temp = tempfile::Builder::new().suffix(".png").tempfile()?;
        image.save_with_format(temp.path(), image::ImageFormat::Png)?;
        let asset = assets::import(&store, temp.path())?;
        let mut character = Character::new(&format!("Personaje {}", i + 1));
        character
            .states
            .insert("idle".into(), Pose::new(asset.id.clone()));
        let uid = format!("100000000000000{:03}", i);
        let name = [
            "Ariadna", "Nico", "Luna", "Santi", "Mara", "Alex", "Teo", "Sol",
        ][i % 8];
        doc.people.insert(
            uid.clone(),
            Person {
                id: uid.clone(),
                name: format!("{name} · {}", i + 1),
                character: Some(character.id.clone()),
                avatar: None,
            },
        );
        doc.profiles[0].groups[0].members.push(Member {
            user: uid,
            ..Default::default()
        });
        doc.assets.insert(asset.id.clone(), asset);
        doc.characters.push(character);
    }
    store.apply(&doc)?;
    println!(
        "http://127.0.0.1:39465/o/{}/group/{}",
        store.token()?,
        doc.profiles[0].groups[0].id
    );
    Ok(())
}
