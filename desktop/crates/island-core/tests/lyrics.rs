//! Port of the plugin's `test/lyrics.test.mjs`: made-up songs and records only.

use island_core::lyrics::*;
use serde_json::{Value, json};

fn times(r: &Lrc) -> Vec<u64> {
    r.lines.iter().map(|l| l.t).collect()
}

fn texts(r: &Lrc) -> Vec<&str> {
    r.lines.iter().map(|l| l.text.as_str()).collect()
}

#[test]
fn parse_time_usual_forms_and_rejects() {
    assert_eq!(parse_time("00:17.12"), Some(17120));
    assert_eq!(parse_time("01:02.345"), Some(62345));
    assert_eq!(parse_time("1:02"), Some(62000));
    assert_eq!(parse_time("00:05.5"), Some(5500));
    assert_eq!(parse_time("1:02:03.50"), Some(3_723_500));
    assert_eq!(parse_time("00:61.00"), None);
    assert_eq!(parse_time("ar:Alguien"), None);
    assert_eq!(parse_time(""), None);
}

#[test]
fn parse_lrc_several_stamps_on_a_line_in_order() {
    let r = parse_lrc("[00:12.00][00:45.50]Estribillo\n[00:20.00]Verso");
    assert_eq!(times(&r), [12000, 20000, 45500]);
    assert_eq!(texts(&r), ["Estribillo", "Verso", "Estribillo"]);
}

#[test]
fn parse_lrc_two_and_three_decimals_none_at_all() {
    let r = parse_lrc("[00:01.5]a\n[00:02.25]b\n[00:03.125]c\n[00:04]d");
    assert_eq!(times(&r), [1500, 2250, 3125, 4000]);
}

#[test]
fn parse_lrc_offset_moves_lines_metadata_ignored() {
    let r = parse_lrc(
        "[ar:Artista Ficticio]\n[ti:Isla de prueba]\n[al:Demo]\n[by:nadie]\n[length:03:00]\n[offset:+300]\n[00:10.00]uno\n[00:20.00]dos",
    );
    assert_eq!(r.offset, 300);
    assert_eq!(times(&r), [9700, 19700]);
    assert_eq!(times(&parse_lrc("[offset:-500]\n[00:10.00]uno")), [10500]);
    // Never before zero.
    assert_eq!(times(&parse_lrc("[offset:5000]\n[00:01.00]x")), [0]);
}

#[test]
fn parse_lrc_empty_lines_stay_word_stamps_and_controls_go() {
    let r = parse_lrc(
        "[00:01.00]hola <00:01.20>mundo <00:01.50>fin\n[00:05.00]\n[00:06.00]\u{1b}[31mrojo\u{200b}\u{7}",
    );
    assert_eq!(texts(&r), ["hola mundo fin", "", "rojo"]);
}

#[test]
fn parse_lrc_crlf_bom_unstamped_lines_and_stable_order() {
    let r = parse_lrc("\u{feff}[00:02.00]b\r\nsin marca\r\n[00:01.00]a\r\n[00:02.00]c");
    assert_eq!(texts(&r), ["a", "b", "c"]);
}

#[test]
fn parse_lrc_caps_on_lines_line_length_and_text_size() {
    let big: String = (0..2000)
        .map(|i| format!("[{:02}:{:02}.00]x\n", i / 60, i % 60))
        .collect();
    assert_eq!(parse_lrc(&big).lines.len(), MAX_LINES);
    let long = format!("[00:01.00]{}", "a".repeat(5000));
    assert_eq!(parse_lrc(&long).lines[0].text.len(), MAX_LINE);
    let huge = format!(
        "[00:01.00]hola\n{}\n[00:09.00]perdida",
        "x".repeat(MAX_TEXT * 2)
    );
    assert_eq!(texts(&parse_lrc(&huge)), ["hola"]);
}

#[test]
fn parse_lrc_cuts_by_utf16_units_without_splitting_characters() {
    // 🎵 is two UTF-16 units: a cut in the middle drops it whole.
    let line = format!("[00:01.00]{}🎵", "a".repeat(MAX_LINE - 1));
    assert_eq!(parse_lrc(&line).lines[0].text, "a".repeat(MAX_LINE - 1));
}

#[test]
fn parse_lrc_empty_gives_nothing() {
    assert_eq!(parse_lrc(""), Lrc::default());
}

fn song() -> Vec<Line> {
    parse_lrc("[00:05.00]uno\n[00:10.00]\n[00:15.00]tres\n[00:20.00]cuatro").lines
}

#[test]
fn line_at_before_exact_between_after() {
    let s = song();
    assert_eq!(line_at(&s, 0), None);
    assert_eq!(line_at(&s, 4999), None);
    assert_eq!(line_at(&s, 5000), Some(0));
    assert_eq!(line_at(&s, 12000), Some(1));
    assert_eq!(line_at(&s, 99999), Some(3));
    assert_eq!(line_at(&[], 5), None);
}

#[test]
fn line_at_agrees_with_a_linear_search() {
    let mut seed: u64 = 7;
    let mut rnd = || {
        seed = (seed.wrapping_mul(1_103_515_245).wrapping_add(12345)) % 2_147_483_648;
        seed
    };
    let mut lines = Vec::new();
    let mut t = 0;
    for i in 0..120 {
        t += rnd() % 6000;
        lines.push(Line {
            t,
            text: format!("l{i}"),
        });
    }
    for _ in 0..200 {
        let pos = rnd() % (t + 5000);
        let want = lines.iter().rposition(|l| l.t <= pos);
        assert_eq!(line_at(&lines, pos), want);
    }
}

#[test]
fn window_skips_gaps_and_ends() {
    let s = song();
    let w = |pos| {
        let w = window(&s, pos);
        (
            w.index,
            w.current.to_string(),
            w.next.to_string(),
            w.next_at,
        )
    };
    assert_eq!(w(2000), (None, "".into(), "uno".into(), Some(5000)));
    assert_eq!(w(6000), (Some(0), "uno".into(), "tres".into(), Some(10000)));
    assert_eq!(w(11000), (Some(1), "".into(), "tres".into(), Some(15000)));
    assert_eq!(w(30000), (Some(3), "cuatro".into(), "".into(), None));
}

#[test]
fn normalize_title_streaming_suffixes_go() {
    for t in [
        "Canción Inventada - Remastered 2011",
        "Canción Inventada - 2009 Remaster",
        "Canción Inventada (Remastered)",
        "Canción Inventada (feat. Otra Persona)",
        "Canción Inventada [ft. Otra Persona]",
        "Canción Inventada ft. Otra Persona",
        "Canción Inventada (Official Video)",
        "Canción Inventada (Official Music Video) [HD]",
        "Canción Inventada (Lyrics)",
        "Canción Inventada (Visualizer)",
        "Canción Inventada (Bonus Track)",
        "Canción Inventada (Deluxe Edition)",
    ] {
        assert_eq!(normalize_title(t), "Canción Inventada", "{t}");
    }
}

#[test]
fn normalize_title_what_changes_the_song_stays() {
    for t in [
        "Canción Inventada (Live)",
        "Canción Inventada (Acoustic)",
        "Canción Inventada (Radio Edit)",
        "Canción Inventada - Live in Madrid",
        "Plain title",
        "(Remastered)",
    ] {
        assert_eq!(normalize_title(t), t);
    }
}

#[test]
fn normalize_artist_first_of_the_list() {
    for a in [
        "Artista Uno, Artista Dos",
        "Artista Uno & Artista Dos",
        "Artista Uno feat. Artista Dos",
        "Artista Uno x Artista Dos",
        "Artista Uno; Artista Dos",
    ] {
        assert_eq!(normalize_artist(a), "Artista Uno", "{a}");
    }
    assert_eq!(normalize_artist("Artista Solo"), "Artista Solo");
}

#[test]
fn cache_key_stable_ignores_case_and_spacing_rounds_duration() {
    let k = cache_key("Artista Ficticio", "Isla de prueba", "Demo", 180.0);
    assert_eq!(k, "artista ficticio\nisla de prueba\ndemo\n180");
    assert_eq!(
        cache_key("  ARTISTA   Ficticio ", "Isla  de PRUEBA", "demo", 179.6),
        k
    );
    assert_ne!(
        cache_key("Artista Ficticio", "Isla de prueba", "Demo", 181.0),
        k
    );
    assert_eq!(cache_key("a", "b", "", 0.0), "a\nb\n\n0");
    assert_eq!(cache_key("a", "b", "", f64::NAN), "a\nb\n\n0");
}

fn rec(extra: Value) -> Value {
    let mut r = json!({
        "id": 42, "trackName": "Isla de prueba", "duration": 180,
        "instrumental": false, "plainLyrics": null, "syncedLyrics": null
    });
    for (k, v) in extra.as_object().unwrap() {
        r[k] = v.clone();
    }
    r
}

#[test]
fn validate_synced_plain_instrumental_and_nothing() {
    let s = validate(
        &rec(json!({"syncedLyrics": "[00:05.00]uno\n[00:10.00]dos", "plainLyrics": "uno\ndos"})),
        180.0,
    );
    assert_eq!(s.kind, Kind::Synced);
    assert_eq!(s.id, 42);
    assert_eq!(s.lines.len(), 2);
    let kind = |extra| validate(&rec(extra), 180.0).kind;
    assert_eq!(kind(json!({"plainLyrics": "uno\ndos"})), Kind::Plain);
    assert_eq!(kind(json!({"instrumental": true})), Kind::Instrumental);
    assert_eq!(kind(json!({})), Kind::Nothing);
    assert_eq!(validate(&json!({}), 180.0).kind, Kind::Nothing);
    // Synced text with no stamps is not synced.
    assert_eq!(kind(json!({"syncedLyrics": "sin marcas"})), Kind::Nothing);
}

#[test]
fn validate_text_accepts_json_rejects_broken_huge_and_wrong_shapes() {
    let ok = rec(json!({"syncedLyrics": "[00:01.00]x"})).to_string();
    assert_eq!(validate_text(&ok, 180.0).kind, Kind::Synced);
    let huge = "x".repeat(300_000);
    for bad in ["{roto", "", "[]", "null", "5", huge.as_str()] {
        assert_eq!(validate_text(bad, 180.0).kind, Kind::Nothing);
    }
    let too_long = format!("[00:01.00]{}", "x".repeat(MAX_TEXT));
    assert_eq!(
        validate(&rec(json!({ "syncedLyrics": too_long })), 180.0).kind,
        Kind::Nothing
    );
}

#[test]
fn validate_far_duration_is_not_used() {
    let r = rec(json!({"syncedLyrics": "[00:01.00]x", "duration": 200}));
    assert_eq!(validate(&r, 180.0).kind, Kind::Nothing);
    assert_eq!(validate(&r, 198.0).kind, Kind::Synced);
}

#[test]
fn pick_search_closest_duration_with_synced_lyrics() {
    let list = json!([
        rec(json!({"id": 1, "duration": 150, "syncedLyrics": "[00:01.00]a"})),
        rec(json!({"id": 2, "duration": 182, "syncedLyrics": "[00:01.00]b"})),
        rec(json!({"id": 3, "duration": 180, "plainLyrics": "solo texto"})),
        rec(json!({"id": 4, "duration": 179, "syncedLyrics": "[00:01.00]c"})),
        null,
        7
    ]);
    assert_eq!(pick_search(&list, 180.0).unwrap()["id"], 4);
    assert!(pick_search(&json!([]), 180.0).is_none());
    assert!(pick_search(&Value::Null, 180.0).is_none());
    let far = json!([rec(json!({"duration": 100, "syncedLyrics": "[00:01.00]a"}))]);
    assert!(pick_search(&far, 180.0).is_none());
}
