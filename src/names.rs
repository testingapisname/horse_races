use rand::seq::SliceRandom;

pub fn random_names() -> Vec<String> {
    let prefixes = vec![
        "Whiskey", "Turbo", "Thunder", "Glue", "Hoof", "Lucky",
        "Shadow", "Biscuit", "Hay", "Neigh", "Midnight", "Clover",
    ];
    let suffixes = vec![
        "Business", "Runner", "Hearted", "Factory", "Strike",
        "Whisper", "McGraw", "Escapee", "Dream", "Storm", "Bolt",
    ];

    let mut rng = rand::thread_rng();
    let mut names = Vec::new();

    for _ in 0..5 {
        let pre = prefixes.choose(&mut rng).unwrap();
        let suf = suffixes.choose(&mut rng).unwrap();
        names.push(format!("{} {}", pre, suf));
    }

    names
}
