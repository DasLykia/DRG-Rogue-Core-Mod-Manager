//! Nexus Mods access: the public GraphQL API for listing, the v1 REST API (personal API key) for downloads.
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;
use std::sync::Arc;
use crate::ui::t;

pub const GAME_DOMAIN: &str = "deeprockgalacticroguecore";
pub const GAME_ID: u32 = 9293;
const GQL: &str = "https://api.nexusmods.com/v2/graphql";
const V1: &str = "https://api.nexusmods.com/v1";

#[derive(Clone, Debug, Default)]
pub struct NexusMod {
    pub id: u32,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub version: String,
    pub category: String,
    pub endorsements: u64,
    pub downloads: u64,
    pub updated: String,
    pub created: String,
    pub adult: bool,
    pub thumb: String,
    pub picture: String,
}

#[derive(Clone, Debug)]
pub struct NexusFile {
    pub id: u32,
    pub name: String,
    pub version: String,
    pub category: String,
    pub size: u64,
    pub description: String,
    /// Upload time (Unix seconds).
    pub date: u64,
    /// Nexus "file group": newer versions of the same file share it.
    pub group: u64,
    /// The file's download name on Nexus, e.g. "DeepRockTimer45-4-1-1780011301.zip".
    pub uri: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NexusUser {
    pub name: String,
    pub is_premium: bool,
}

pub fn agent() -> ureq::Agent {
    let tls = native_tls::TlsConnector::new().expect("TLS");
    ureq::AgentBuilder::new()
        .tls_connector(Arc::new(tls))
        .user_agent("CoreModManager/0.1")
        .build()
}

fn gql(query: &str) -> Result<Value, String> {
    let v: Value = agent()
        .post(GQL)
        .send_json(json!({ "query": query }))
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;
    if let Some(err) = v.get("errors") {
        return Err(err.to_string());
    }
    Ok(v["data"].clone())
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

/// Every mod of the game (137 at the time of writing), fetched in pages of 100.
pub fn fetch_all_mods() -> Result<Vec<NexusMod>, String> {
    let mut out = Vec::new();
    loop {
        let q = format!(
            r#"query {{ mods(filter: {{ gameDomainName: [{{ value: "{GAME_DOMAIN}" }}] }}, sort: [{{ endorsements: {{ direction: DESC }} }}], count: 100, offset: {}) {{ totalCount nodes {{ modId name summary author uploader {{ name }} endorsements downloads version updatedAt createdAt adultContent pictureUrl thumbnailUrl modCategory {{ name }} }} }} }}"#,
            out.len()
        );
        let d = gql(&q)?;
        let total = d["mods"]["totalCount"].as_u64().unwrap_or(0) as usize;
        let nodes = d["mods"]["nodes"].as_array().cloned().unwrap_or_default();
        if nodes.is_empty() {
            break;
        }
        for n in nodes {
            let author = if s(&n["author"]).trim().is_empty() { s(&n["uploader"]["name"]) } else { s(&n["author"]).trim().to_string() };
            out.push(NexusMod {
                id: n["modId"].as_u64().unwrap_or(0) as u32,
                name: s(&n["name"]),
                summary: html_unescape(&s(&n["summary"])),
                author,
                version: s(&n["version"]),
                category: s(&n["modCategory"]["name"]),
                endorsements: n["endorsements"].as_u64().unwrap_or(0),
                downloads: n["downloads"].as_u64().unwrap_or(0),
                updated: s(&n["updatedAt"]),
                created: s(&n["createdAt"]),
                adult: n["adultContent"].as_bool().unwrap_or(false),
                thumb: s(&n["thumbnailUrl"]),
                picture: s(&n["pictureUrl"]),
            });
        }
        if out.len() >= total {
            break;
        }
    }
    Ok(out)
}

/// Long description (BBCode stripped) and the downloadable files of one mod.
const FILE_FIELDS: &str = "fileId name version category sizeInBytes description date groupId uri";

fn parse_file(f: &Value) -> NexusFile {
    NexusFile {
        id: f["fileId"].as_u64().unwrap_or(0) as u32,
        name: s(&f["name"]),
        version: s(&f["version"]),
        category: s(&f["category"]),
        size: s(&f["sizeInBytes"]).parse().unwrap_or(0),
        description: strip_bbcode(&s(&f["description"])),
        date: f["date"].as_u64().unwrap_or(0),
        group: f["groupId"].as_u64().unwrap_or(0),
        uri: s(&f["uri"]),
    }
}

/// The files of several mods in one request (for checking installed mods for updates).
pub fn fetch_files(mod_ids: &[u32]) -> Result<std::collections::HashMap<u32, Vec<NexusFile>>, String> {
    let mut out = std::collections::HashMap::new();
    // shortcut: 50 aliases per request keeps the query small; more mods just take more requests.
    for chunk in mod_ids.chunks(50) {
        let parts: String = chunk.iter().map(|id| format!("m{id}: modFiles(modId: {id}, gameId: {GAME_ID}) {{ {FILE_FIELDS} }} ")).collect();
        let d = gql(&format!("query {{ {parts}}}"))?;
        for id in chunk {
            let files = d[format!("m{id}")].as_array().map(|a| a.iter().map(parse_file).collect()).unwrap_or_default();
            out.insert(*id, files);
        }
    }
    Ok(out)
}

/// Whether a file that appeared in the Downloads folder is Nexus file `uri` of mod `mod_id`.
/// Browsers add " (1)" to repeated downloads; unfinished downloads are never a match.
pub fn is_nexus_download(file_name: &str, uri: &str, mod_id: u32) -> bool {
    let lower = file_name.to_lowercase();
    if [".crdownload", ".part", ".tmp", ".download", ".partial"].iter().any(|x| lower.ends_with(x)) {
        return false;
    }
    let base = |n: &str| -> String {
        let n = n.to_lowercase();
        let (stem, ext) = n.rsplit_once('.').unwrap_or((n.as_str(), ""));
        let stem = match stem.rfind(" (") {
            Some(i) if stem.ends_with(')') => &stem[..i],
            _ => stem,
        };
        format!("{stem}.{ext}")
    };
    if !uri.is_empty() {
        return base(file_name) == base(uri);
    }
    lower.contains(&format!("-{mod_id}-"))
}

/// A file that is still offered (not archived, old or deleted).
pub fn is_live(f: &NexusFile) -> bool {
    !matches!(f.category.as_str(), "ARCHIVED" | "OLD_VERSION" | "DELETED" | "REMOVED")
}

/// The newest file that updates what's installed: a newer live file in the installed file's
/// group when the installed file is known; otherwise, for a .pak found in the game folder,
/// the newest main file when its version differs from the version in the .pak's name.
pub fn pending_update<'a>(files: &'a [NexusFile], installed_file: Option<u32>, pak_names: &[String]) -> Option<&'a NexusFile> {
    let newest = |it: Box<dyn Iterator<Item = &'a NexusFile> + 'a>| it.max_by_key(|f| f.date);
    match installed_file.and_then(|id| files.iter().find(|f| f.id == id)) {
        Some(cur) => newest(Box::new(files.iter().filter(move |f| is_live(f) && f.id != cur.id && f.date > cur.date && (cur.group == 0 || f.group == cur.group)))),
        None if installed_file.is_some() => newest(Box::new(files.iter().filter(|f| is_live(f) && f.category == "MAIN"))),
        None => {
            let local = pak_names.iter().find_map(|n| version_in_name(n))?;
            let latest = newest(Box::new(files.iter().filter(|f| is_live(f) && matches!(f.category.as_str(), "MAIN" | "UPDATE"))))?;
            (norm_version(&latest.version) != norm_version(&local)).then_some(latest)
        }
    }
}

/// "WorkbenchFullControlV2.3_P.pak" -> "2.3", "HeatCrosshair-1.0.0_P.pak" -> "1.0.0", "Bhop_P.pak" -> None.
pub fn version_in_name(name: &str) -> Option<String> {
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    let stem = stem.strip_suffix("_P").or_else(|| stem.strip_suffix("_p")).unwrap_or(stem);
    let b = stem.as_bytes();
    let mut best = None;
    for i in 0..b.len() {
        let starts = b[i].is_ascii_digit() && (i == 0 || !b[i - 1].is_ascii_digit() && b[i - 1] != b'.');
        // A version is "v1"/"V2.3" or a dotted number like "1.0.0", after a separator or the letter v.
        if !starts {
            continue;
        }
        let after_v = i > 0 && (b[i - 1] == b'v' || b[i - 1] == b'V');
        let end = (i..b.len()).find(|&j| !(b[j].is_ascii_digit() || b[j] == b'.')).unwrap_or(b.len());
        let v = stem[i..end].trim_end_matches('.');
        if after_v || v.contains('.') {
            best = Some(v.to_string());
        }
    }
    best
}

/// "v2.30" and "2.3.0" compare equal to "2.3".
fn norm_version(v: &str) -> String {
    let parts: Vec<String> = v.trim().trim_start_matches(['v', 'V']).split('.').map(|p| p.trim_start_matches('0').to_string()).collect();
    let mut parts: Vec<String> = parts.into_iter().map(|p| if p.is_empty() { "0".into() } else { p }).collect();
    while parts.len() > 1 && parts.last().map_or(false, |p| p == "0") {
        parts.pop();
    }
    parts.join(".")
}

pub fn fetch_detail(mod_id: u32) -> Result<(String, Vec<NexusFile>), String> {
    let q = format!(
        r#"query {{ m: mod(modId: {mod_id}, gameId: {GAME_ID}) {{ description }} f: modFiles(modId: {mod_id}, gameId: {GAME_ID}) {{ {FILE_FIELDS} }} }}"#
    );
    let d = gql(&q)?;
    let desc = strip_bbcode(&s(&d["m"]["description"]));
    let mut files: Vec<NexusFile> = d["f"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(parse_file)
        .collect();
    // MAIN first, then OPTIONAL, UPDATE, MISCELLANEOUS; ARCHIVED last; newest id first inside a group.
    let rank = |c: &str| match c { "MAIN" => 0, "OPTIONAL" => 1, "UPDATE" => 2, "ARCHIVED" | "OLD_VERSION" => 9, _ => 3 };
    files.sort_by(|a, b| rank(&a.category).cmp(&rank(&b.category)).then(b.id.cmp(&a.id)));
    Ok((desc, files))
}

pub fn validate_key(key: &str) -> Result<NexusUser, String> {
    agent()
        .get(&format!("{V1}/users/validate.json"))
        .set("apikey", key)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(401, _) => t("API-Schlüssel ungültig", "Invalid API key").to_string(),
            e => e.to_string(),
        })?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Download URL for a file. Premium accounts need only the key; free accounts need the
/// key/expires pair from a "Mod Manager Download" (nxm://) link.
pub fn download_url(key: &str, mod_id: u32, file_id: u32, nxm: Option<(&str, &str)>) -> Result<String, String> {
    let mut url = format!("{V1}/games/{GAME_DOMAIN}/mods/{mod_id}/files/{file_id}/download_link.json");
    if let Some((k, e)) = nxm {
        url.push_str(&format!("?key={k}&expires={e}"));
    }
    let v: Value = agent()
        .get(&url)
        .set("apikey", key)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(403, _) => t("Direkter Download nur mit Nexus Premium. Nutze \"Mod Manager Download\" auf Nexus.", "Direct downloads need Nexus Premium. Use \"Mod Manager Download\" on Nexus.").to_string(),
            ureq::Error::Status(401, _) => t("API-Schlüssel ungültig", "Invalid API key").to_string(),
            e => e.to_string(),
        })?
        .into_json()
        .map_err(|e| e.to_string())?;
    v.get(0).and_then(|x| x["URI"].as_str()).map(String::from).ok_or_else(|| t("Kein Download-Link erhalten", "No download link received").into())
}

/// Streams a URL to `dest`, reporting (done, total) bytes.
pub fn download_to(url: &str, dest: &std::path::Path, mut progress: impl FnMut(u64, u64)) -> Result<(), String> {
    let resp = agent().get(url).call().map_err(|e| e.to_string())?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut reader = resp.into_reader();
    let tmp = dest.with_extension("part");
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut f, &buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        progress(done, total);
    }
    drop(f);
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// The game's latest public patch.
#[derive(Clone, Debug, Default)]
pub struct GamePatch {
    /// Steam build id of the public branch ("" when unknown).
    pub buildid: String,
    /// When that build went public (Unix seconds).
    pub released: u64,
    /// Version from the patch notes title, e.g. "EA 03.04.00" ("" when unknown).
    pub name: String,
}

/// Public build from steamcmd.net and the version name from Steam's own patch-notes feed;
/// either may fail, the other still counts.
pub fn fetch_game_patch() -> Result<GamePatch, String> {
    let mut p = GamePatch::default();
    let build = agent().get("https://api.steamcmd.net/v1/info/2605790").call().map_err(|e| e.to_string()).and_then(|r| r.into_json::<Value>().map_err(|e| e.to_string()));
    if let Ok(v) = &build {
        let b = &v["data"]["2605790"]["depots"]["branches"]["public"];
        p.buildid = s(&b["buildid"]);
        p.released = s(&b["timeupdated"]).parse().unwrap_or(0);
    }
    let news = agent()
        .get("https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/?appid=2605790&count=20&maxlength=1&feeds=steam_community_announcements")
        .call()
        .map_err(|e| e.to_string())
        .and_then(|r| r.into_json::<Value>().map_err(|e| e.to_string()));
    if let Ok(v) = &news {
        let items = v["appnews"]["newsitems"].as_array().cloned().unwrap_or_default();
        if let Some((name, date)) = items.iter().find_map(|n| patch_name(&s(&n["title"])).map(|name| (name, n["date"].as_u64().unwrap_or(0)))) {
            p.name = name;
            if p.released == 0 {
                p.released = date;
            }
        }
    }
    match (build, news) {
        (Err(a), Err(b)) => Err(format!("{a}; {b}")),
        _ => Ok(p),
    }
}

/// "PATCH NOTES - RC: EA:03:04:00" -> "EA 03.04.00"; None for posts that aren't patches.
fn patch_name(title: &str) -> Option<String> {
    let up = title.to_uppercase();
    if !(up.contains("PATCH NOTES") || up.contains("HOT FIX") || up.contains("HOTFIX")) {
        return None;
    }
    let i = up.find("EA")?;
    let ver: String = up[i + 2..].trim_start_matches([':', ' ']).chars().take_while(|c| c.is_ascii_digit() || *c == ':' || *c == '.').collect();
    (!ver.is_empty()).then(|| format!("EA {}", ver.replace(':', ".")))
}

pub const WIKI_MAIN: &str = "https://deeprockgalactic.wiki.gg/wiki/Rogue_Core:Main";

/// Searches the wiki's "Rogue Core" namespace (6000); returns (page title, plain snippet).
pub fn wiki_search(q: &str) -> Result<Vec<(String, String)>, String> {
    let v: Value = agent()
        .get("https://deeprockgalactic.wiki.gg/api.php")
        .query("action", "query")
        .query("list", "search")
        .query("srsearch", q)
        .query("srnamespace", "6000")
        .query("srlimit", "20")
        .query("format", "json")
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;
    let hits = v["query"]["search"].as_array().cloned().unwrap_or_default();
    Ok(hits.iter().map(|h| (s(&h["title"]), html_unescape(&strip_tags(&s(&h["snippet"]))))).collect())
}

fn strip_tags(t: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in t.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    out
}

/// Web address of a wiki page title ("Rogue Core:Bio Booster Decks").
pub fn wiki_url(title: &str) -> String {
    let mut out = String::from("https://deeprockgalactic.wiki.gg/wiki/");
    for b in title.replace(' ', "_").bytes() {
        if b.is_ascii_alphanumeric() || b"-_.:/()',".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    agent().get(url).call().ok()?.into_reader().take(20 << 20).read_to_end(&mut v).ok()?;
    Some(v)
}

/// Parsed `nxm://deeprockgalacticroguecore/mods/4/files/53?key=..&expires=..&user_id=..`
#[derive(Clone, Debug, PartialEq)]
pub struct NxmLink {
    pub mod_id: u32,
    pub file_id: u32,
    pub key: String,
    pub expires: String,
}

pub fn parse_nxm(url: &str) -> Option<NxmLink> {
    let rest = url.strip_prefix("nxm://")?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    if parts.len() < 5 || !parts[0].eq_ignore_ascii_case(GAME_DOMAIN) || parts[1] != "mods" || parts[3] != "files" {
        return None;
    }
    let get = |k: &str| query.split('&').find_map(|kv| kv.strip_prefix(&format!("{k}="))).unwrap_or("").to_string();
    Some(NxmLink { mod_id: parts[2].parse().ok()?, file_id: parts[4].parse().ok()?, key: get("key"), expires: get("expires") })
}

pub fn html_unescape(t: &str) -> String {
    let mut out = t
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#92;", "\\")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    out = out.replace("&amp;", "&");
    out
}

/// Removes [tag=..] and [/tag] markup; keeps text and link targets readable.
pub fn strip_bbcode(t: &str) -> String {
    let t = html_unescape(t);
    let mut out = String::with_capacity(t.len());
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let tag: String = chars.clone().take_while(|&x| x != ']' && x != '[' && x != '\n').collect();
            let looks_like_tag = tag.len() < 60
                && chars.clone().nth(tag.chars().count()) == Some(']')
                && tag.trim_start_matches('/').chars().next().map_or(false, |x| x.is_ascii_alphabetic() || x == '*');
            if looks_like_tag {
                for _ in 0..=tag.chars().count() {
                    chars.next();
                }
                if tag == "*" {
                    out.push_str("• ");
                } else if let Some(u) = tag.strip_prefix("url=") {
                    out.push_str(&format!("({u}) "));
                }
                continue;
            }
        }
        out.push(c);
    }
    let mut res = String::new();
    let mut blank = 0;
    for line in out.lines() {
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        res.push_str(line.trim_end());
        res.push('\n');
    }
    res.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nxm_and_bbcode() {
        let l = parse_nxm("nxm://deeprockgalacticroguecore/mods/4/files/53?key=abc&expires=99&user_id=1").unwrap();
        assert_eq!((l.mod_id, l.file_id, l.key.as_str(), l.expires.as_str()), (4, 53, "abc", "99"));
        assert!(parse_nxm("nxm://skyrim/mods/4/files/53").is_none());
        assert_eq!(wiki_url("Rogue Core:\"Decker\" SMG"), "https://deeprockgalactic.wiki.gg/wiki/Rogue_Core:%22Decker%22_SMG");
        assert_eq!(patch_name("PATCH NOTES - RC: EA:03:04:00").as_deref(), Some("EA 03.04.00"));
        assert!(is_nexus_download("DeepRockTimer45-4-1-1780011301.zip", "DeepRockTimer45-4-1-1780011301.zip", 4));
        assert!(is_nexus_download("DeepRockTimer45-4-1-1780011301 (1).zip", "DeepRockTimer45-4-1-1780011301.zip", 4));
        assert!(!is_nexus_download("DeepRockTimer75-4-1-1780011316.zip", "DeepRockTimer45-4-1-1780011301.zip", 4));
        assert!(!is_nexus_download("DeepRockTimer45-4-1-1780011301.zip.crdownload", "", 4));
        assert!(is_nexus_download("Something-4-1-1.zip", "", 4));
        // Update detection.
        assert_eq!(version_in_name("WorkbenchFullControlV2.3_P.pak").as_deref(), Some("2.3"));
        assert_eq!(version_in_name("HeatCrosshair-1.0.0_P.pak").as_deref(), Some("1.0.0"));
        assert_eq!(version_in_name("BringBackGrapplingHook_v1_P.pak").as_deref(), Some("1"));
        assert_eq!(version_in_name("ModHub_P.pak"), None);
        assert_eq!(norm_version("v2.30"), "2.30");
        assert_eq!(norm_version("2.3.0"), "2.3");
        let f = |id, ver: &str, cat: &str, date, group| NexusFile { id, name: String::new(), version: ver.into(), category: cat.into(), size: 0, description: String::new(), date, group, uri: String::new() };
        let files = vec![f(289, "2", "OLD_VERSION", 10, 7), f(292, "2.2", "OLD_VERSION", 20, 7), f(297, "2.3", "MAIN", 30, 7), f(300, "1", "MAIN", 40, 8)];
        assert_eq!(pending_update(&files, Some(292), &[]).map(|x| x.id), Some(297));
        assert_eq!(pending_update(&files, Some(297), &[]).map(|x| x.id), None);
        assert_eq!(pending_update(&files, None, &["WorkbenchFullControlV2.3_P.pak".into()]).map(|x| x.id), Some(300));
        assert_eq!(pending_update(&files[..3], None, &["WorkbenchFullControlV2.3_P.pak".into()]).map(|x| x.id), None);
        assert_eq!(pending_update(&files[..3], None, &["ModHub_P.pak".into()]).map(|x| x.id), None);
        assert_eq!(patch_name("HOT FIX: RC:EA:03.00.01").as_deref(), Some("EA 03.00.01"));
        assert_eq!(patch_name("Rogue Core’s FREE WEEKEND is here"), None);
        assert_eq!(strip_tags("a <span class=\"x\">b</span> c"), "a b c");
        assert_eq!(strip_bbcode("[size=4][b]Hi[/b][/size] [1] x&#92;y"), "Hi [1] x\\y");
    }
}
