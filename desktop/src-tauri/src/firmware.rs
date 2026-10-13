//! Published firmware: version compare, the public manifest, and the payment seam.

use serde::Deserialize;
use serde::Serialize;

use crate::samba::{APP_OFFSET, APP_ROOM};
use crate::sha256;

/// Fallback when the GitHub API is unavailable. Branch paths on this CDN can lag for minutes.
pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/yortyrh/liveuse-releases/HEAD/manifest.json";
const MANIFEST_API_URL: &str =
    "https://api.github.com/repos/yortyrh/liveuse-releases/contents/manifest.json";
const GITHUB_USER_AGENT: &str = "LiveUse-Desktop";
pub const PAYMENT_REQUIRED: &str = "payment_required";

const ASSET_HOST: &str = "https://github.com/yortyrh/liveuse-releases/releases/download/";
const RUNNING_VID: u16 = 0x2886;
const RUNNING_PID: u16 = 0x802F;
const BOOTLOADER_VID: u16 = 0x2886;
const BOOTLOADER_PID: u16 = 0x002F;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for Version {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(Version {
        major,
        minor,
        patch,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlashKind {
    Update { from: Option<Version>, to: Version },
    Initial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entitlement {
    Allowed,
    /// Not produced today. `authorize` returns `Allowed` for both kinds;
    /// a later version returns this for `FlashKind::Initial`.
    #[cfg_attr(not(test), expect(dead_code))]
    PaymentRequired,
}

pub fn authorize(kind: &FlashKind) -> Entitlement {
    match kind {
        FlashKind::Update { .. } => Entitlement::Allowed,
        // Primera imagen sobre la región vacía. Hoy pasa.
        // Una versión futura cambia solo este brazo.
        FlashKind::Initial => Entitlement::Allowed,
    }
}

pub fn rejection(entitlement: Entitlement) -> Result<(), String> {
    match entitlement {
        Entitlement::Allowed => Ok(()),
        Entitlement::PaymentRequired => Err(PAYMENT_REQUIRED.to_string()),
    }
}

pub fn authorize_or_reject(kind: &FlashKind) -> Result<(), String> {
    rejection(authorize(kind))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedBoard {
    pub version: Version,
    pub notes: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    version: String,
    notes: String,
    boards: Vec<ManifestBoard>,
}

#[derive(Debug, Deserialize)]
struct ManifestBoard {
    url: String,
    sha256: String,
    size: u64,
    usb: ManifestUsb,
    flash: ManifestFlash,
}

#[derive(Debug, Deserialize)]
struct ManifestUsb {
    running: ManifestId,
    bootloader: ManifestId,
}

#[derive(Debug, Deserialize)]
struct ManifestId {
    vid: String,
    pid: String,
}

#[derive(Debug, Deserialize)]
struct ManifestFlash {
    protocol: String,
    enter: String,
    offset: String,
}

pub fn matching_board(text: &str) -> Result<Option<AcceptedBoard>, String> {
    let manifest: ManifestFile =
        serde_json::from_str(text).map_err(|err| format!("manifest: {err}"))?;
    let Some(version) = parse_version(&manifest.version) else {
        return Ok(None);
    };
    let notes = manifest.notes;
    Ok(manifest
        .boards
        .into_iter()
        .find_map(|board| accept_board(version, &notes, board)))
}

fn accept_board(version: Version, notes: &str, board: ManifestBoard) -> Option<AcceptedBoard> {
    if parse_id(&board.usb.running.vid)? != RUNNING_VID
        || parse_id(&board.usb.running.pid)? != RUNNING_PID
        || parse_id(&board.usb.bootloader.vid)? != BOOTLOADER_VID
        || parse_id(&board.usb.bootloader.pid)? != BOOTLOADER_PID
    {
        return None;
    }
    if board.flash.protocol != "sam-ba" || board.flash.enter != "1200bps-touch" {
        return None;
    }
    if parse_hex_u32(&board.flash.offset)? != APP_OFFSET {
        return None;
    }
    if board.size == 0 || board.size > APP_ROOM as u64 {
        return None;
    }
    if !hex64(&board.sha256) {
        return None;
    }
    if !asset_url_ok(&board.url, &version) {
        return None;
    }
    Some(AcceptedBoard {
        version,
        notes: notes.to_string(),
        url: board.url,
        sha256: board.sha256.to_ascii_lowercase(),
        size: board.size,
    })
}

pub fn asset_url_ok(url: &str, version: &Version) -> bool {
    let prefix = format!("{ASSET_HOST}firmware-v{version}/");
    let Some(rest) = url.strip_prefix(&prefix) else {
        return false;
    };
    !rest.is_empty() && !rest.contains("..") && !rest.contains('\\') && !rest.contains('?')
}

fn parse_id(text: &str) -> Option<u16> {
    let text = text.trim();
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    u16::from_str_radix(hex, 16).ok()
}

fn parse_hex_u32(text: &str) -> Option<u32> {
    let hex = text
        .trim()
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    u32::from_str_radix(hex, 16).ok()
}

fn hex64(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn image_matches(bytes: &[u8], expected_sha: &str, expected_size: u64) -> Result<(), String> {
    if bytes.len() as u64 != expected_size {
        return Err("The firmware file size does not match the manifest.".into());
    }
    if bytes.is_empty() || bytes.len() > APP_ROOM {
        return Err("The firmware image does not fit the XIAO application region.".into());
    }
    let actual = sha256::hex(&sha256::digest(bytes));
    if actual != expected_sha.to_ascii_lowercase() {
        return Err("The firmware file hash does not match the manifest.".into());
    }
    Ok(())
}

async fn fetch_manifest_text(client: &reqwest::Client) -> Result<String, String> {
    if let Ok(text) = fetch_manifest_via_api(client).await {
        return Ok(text);
    }
    fetch_manifest_via_raw(client).await
}

async fn fetch_manifest_via_api(client: &reqwest::Client) -> Result<String, String> {
    client
        .get(MANIFEST_API_URL)
        .header(reqwest::header::USER_AGENT, GITHUB_USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/vnd.github.raw")
        .send()
        .await
        .map_err(|err| err.to_string())?
        .error_for_status()
        .map_err(|err| err.to_string())?
        .text()
        .await
        .map_err(|err| err.to_string())
}

async fn fetch_manifest_via_raw(client: &reqwest::Client) -> Result<String, String> {
    let url = format!(
        "{MANIFEST_URL}?t={}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    );
    client
        .get(&url)
        .header(reqwest::header::USER_AGENT, GITHUB_USER_AGENT)
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .header(reqwest::header::PRAGMA, "no-cache")
        .send()
        .await
        .map_err(|err| err.to_string())?
        .error_for_status()
        .map_err(|err| err.to_string())?
        .text()
        .await
        .map_err(|err| err.to_string())
}

pub async fn fetch_manifest() -> Result<AcceptedBoard, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|err| err.to_string())?;
    let text = fetch_manifest_text(&client).await?;
    matching_board(&text)?.ok_or_else(|| "No XIAO firmware in the published manifest.".into())
}

pub async fn download_image(board: &AcceptedBoard) -> Result<Vec<u8>, String> {
    if !asset_url_ok(&board.url, &board.version) {
        return Err("The firmware URL is not the published release.".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|err| err.to_string())?;
    let bytes = client
        .get(&board.url)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .error_for_status()
        .map_err(|err| err.to_string())?
        .bytes()
        .await
        .map_err(|err| err.to_string())?;
    let bytes = bytes.to_vec();
    image_matches(&bytes, &board.sha256, board.size)?;
    Ok(bytes)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub state: String,
    pub current: Option<String>,
    pub available: Option<String>,
    pub notes: Option<String>,
    pub message: Option<String>,
}

impl Offer {
    pub fn none() -> Self {
        Self {
            state: "none".into(),
            current: None,
            available: None,
            notes: None,
            message: None,
        }
    }

    pub fn check_failed(message: impl Into<String>) -> Self {
        Self {
            state: "checkFailed".into(),
            current: None,
            available: None,
            notes: None,
            message: Some(message.into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct OfferCache {
    pub key: String,
    pub offer: Offer,
}

pub fn cache_key(
    connected: bool,
    firmware: Option<&str>,
    bootloader: bool,
    foreign: bool,
    path: Option<&str>,
) -> String {
    if connected {
        format!("run:{}", firmware.unwrap_or(""))
    } else if bootloader {
        format!("boot:{}", path.unwrap_or(""))
    } else if foreign {
        format!("foreign:{}", path.unwrap_or(""))
    } else {
        "none".into()
    }
}

pub fn offer_for_running(current: &str, board: &AcceptedBoard) -> Offer {
    let published = board.version.to_string();
    let Some(version) = parse_version(current) else {
        return Offer {
            state: "checkFailed".into(),
            current: Some(current.to_string()),
            available: Some(published),
            notes: None,
            message: Some("The firmware version on the board can't be compared.".into()),
        };
    };
    if version < board.version {
        Offer {
            state: "update".into(),
            current: Some(version.to_string()),
            available: Some(published),
            notes: Some(board.notes.clone()),
            message: None,
        }
    } else if version == board.version {
        Offer {
            state: "upToDate".into(),
            current: Some(version.to_string()),
            available: Some(published),
            notes: None,
            message: Some("Firmware is up to date.".into()),
        }
    } else {
        Offer {
            state: "deviceNewer".into(),
            current: Some(version.to_string()),
            available: Some(published),
            notes: None,
            message: Some("This board is newer than the published release.".into()),
        }
    }
}

pub fn offer_for_bootloader(board: &AcceptedBoard) -> Offer {
    Offer {
        state: "bootloader".into(),
        current: None,
        available: Some(board.version.to_string()),
        notes: Some(board.notes.clone()),
        message: Some(
            "The board is in the bootloader. Writing firmware restarts it to see whether a sketch is already installed."
                .into(),
        ),
    }
}

pub fn offer_for_foreign(board: &AcceptedBoard) -> Offer {
    Offer {
        state: "foreign".into(),
        current: None,
        available: Some(board.version.to_string()),
        notes: Some(board.notes.clone()),
        message: Some(
            "This board is not running LiveUse. Writing replaces the program on it.".into(),
        ),
    }
}

/// Latest release from the public manifest when no board is connected to compare.
pub fn offer_for_published(board: &AcceptedBoard) -> Offer {
    let published = board.version.to_string();
    Offer {
        state: "published".into(),
        current: None,
        available: Some(published.clone()),
        notes: Some(board.notes.clone()),
        message: Some(format!("Latest published firmware is {published}.")),
    }
}

/// What the flasher saw after releasing the port, without opening at 1200 baud.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seen {
    LiveUse(Version),
    Foreign,
    Bootloader,
    Absent,
}

/// The next step. A bootloader sighting is not a write until a reset has already been tried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Update { from: Version },
    FirstSetup,
    ConfirmForeign,
    ReplaceForeign,
    UpToDate,
    DeviceNewer,
    LeaveBootloader,
    PhysicalSteps,
    NotConnected,
}

pub const PHYSICAL_STEPS: &str = "\
The app couldn't restart the board out of the bootloader, so it did not write anything. \
Close any program using the serial port. Unplug the USB cable and plug it back in, then try again. \
If the board is still in the bootloader, short the two RST pads once — one tap, not two — and try again.";

/// `identity` is the USB name check. A parsed `fw=` version is LiveUse even when
/// the USB strings say something else. Silence from a LiveUse name is a failed
/// read, not another program.
pub fn seen_from_status(
    identity: Option<bool>,
    version: Option<Version>,
) -> Result<Seen, &'static str> {
    match version {
        Some(version) => Ok(Seen::LiveUse(version)),
        None if identity == Some(true) => Err("The board did not answer. Try the update again."),
        None => Ok(Seen::Foreign),
    }
}

pub fn decide(seen: Seen, published: Version, confirmed: bool, tried_leave: bool) -> Decision {
    match seen {
        Seen::Absent if tried_leave => Decision::PhysicalSteps,
        Seen::Absent => Decision::NotConnected,
        Seen::LiveUse(version) if version < published => Decision::Update { from: version },
        Seen::LiveUse(version) if version == published => Decision::UpToDate,
        Seen::LiveUse(_) => Decision::DeviceNewer,
        Seen::Foreign if confirmed => Decision::ReplaceForeign,
        Seen::Foreign => Decision::ConfirmForeign,
        Seen::Bootloader if tried_leave => Decision::FirstSetup,
        Seen::Bootloader => Decision::LeaveBootloader,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AcceptedBoard, Decision, Entitlement, FlashKind, PAYMENT_REQUIRED, Seen, Version,
        authorize, authorize_or_reject, decide, matching_board, offer_for_bootloader,
        offer_for_foreign, offer_for_published, offer_for_running, parse_version, rejection,
        seen_from_status,
    };

    const EXAMPLE: &str = r#"{
      "schema": 1,
      "version": "0.4.1",
      "tag": "firmware-v0.4.1",
      "notes": "changelog",
      "boards": [
        {
          "id": "seeed_xiao",
          "name": "Seeed XIAO SAMD21",
          "file": "liveuse-seeed_xiao-0.4.1.bin",
          "url": "https://github.com/yortyrh/liveuse-releases/releases/download/firmware-v0.4.1/liveuse-seeed_xiao-0.4.1.bin",
          "sha256": "<hex>",
          "size": 0,
          "usb": {
            "running": { "vid": "0x2886", "pid": "0x802F" },
            "bootloader": { "vid": "0x2886", "pid": "0x002F" }
          },
          "flash": {
            "protocol": "sam-ba",
            "enter": "1200bps-touch",
            "offset": "0x2000"
          }
        }
      ]
    }"#;

    fn valid_manifest() -> String {
        EXAMPLE
            .replace(
                "\"sha256\": \"<hex>\"",
                &format!("\"sha256\": \"{}\"", "ab".repeat(32)),
            )
            .replace("\"size\": 0", "\"size\": 1024")
    }

    fn sample_board() -> AcceptedBoard {
        AcceptedBoard {
            version: parse_version("0.5.0").unwrap(),
            notes: "notes".into(),
            url: "https://github.com/yortyrh/liveuse-releases/releases/download/firmware-v0.5.0/liveuse-seeed_xiao-0.5.0.bin".into(),
            sha256: "ab".repeat(32),
            size: 1024,
        }
    }

    #[test]
    fn version_order_rejects_non_semver() {
        let older = parse_version("0.4.1").unwrap();
        let middle = parse_version("0.5.0").unwrap();
        let newer = parse_version("0.5.1").unwrap();
        assert!(older < middle && middle < newer);
        assert!(parse_version("0.5").is_none());
        assert!(parse_version("v0.5.1").is_none());
        assert!(parse_version("0.5.1-beta").is_none());
        assert!(offer_for_running("beta", &sample_board()).state == "checkFailed");
    }

    #[test]
    fn example_manifest_parses_and_contract_is_strict() {
        assert!(matching_board(EXAMPLE).unwrap().is_none());

        let board = matching_board(&valid_manifest()).unwrap().unwrap();
        assert_eq!(
            board.version,
            Version {
                major: 0,
                minor: 4,
                patch: 1
            }
        );

        assert!(
            matching_board(&valid_manifest().replace("0x2000", "0x0"))
                .unwrap()
                .is_none()
        );
        assert!(
            matching_board(&valid_manifest().replace("sam-ba", "esptool"))
                .unwrap()
                .is_none()
        );
        assert!(
            matching_board(&valid_manifest().replace(
                "https://github.com/yortyrh/liveuse-releases/releases/download/",
                "https://example.com/"
            ))
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn authorize_allows_both_kinds_and_payment_code_is_stable() {
        let version = parse_version("0.5.1").unwrap();
        assert_eq!(
            authorize(&FlashKind::Update {
                from: Some(parse_version("0.4.1").unwrap()),
                to: version
            }),
            Entitlement::Allowed
        );
        assert_eq!(authorize(&FlashKind::Initial), Entitlement::Allowed);
        assert!(authorize_or_reject(&FlashKind::Initial).is_ok());
        assert_eq!(
            rejection(Entitlement::PaymentRequired).unwrap_err(),
            PAYMENT_REQUIRED
        );
        assert_eq!(PAYMENT_REQUIRED, "payment_required");
    }

    #[test]
    fn running_and_empty_board_pick_different_offers() {
        let board = sample_board();
        assert_eq!(offer_for_running("0.4.1", &board).state, "update");
        assert_eq!(offer_for_running("0.5.0", &board).state, "upToDate");
        assert_eq!(offer_for_running("0.5.1", &board).state, "deviceNewer");
    }

    #[test]
    fn bootloader_and_foreign_offers_are_distinct() {
        let board = sample_board();
        assert_eq!(offer_for_bootloader(&board).state, "bootloader");
        assert_eq!(offer_for_foreign(&board).state, "foreign");
        assert_eq!(offer_for_published(&board).state, "published");
        assert_eq!(offer_for_published(&board).available.as_deref(), Some("0.5.0"));
        assert!(
            offer_for_bootloader(&board)
                .message
                .unwrap()
                .contains("bootloader")
        );
    }

    #[test]
    fn liveuse_name_without_status_is_not_foreign() {
        let older = parse_version("0.5.1").unwrap();
        assert_eq!(
            seen_from_status(Some(true), Some(older)).unwrap(),
            Seen::LiveUse(older)
        );
        assert!(seen_from_status(Some(true), None).is_err());
        assert_eq!(seen_from_status(Some(false), None).unwrap(), Seen::Foreign);
        assert_eq!(seen_from_status(None, None).unwrap(), Seen::Foreign);
        assert_eq!(
            seen_from_status(Some(false), Some(older)).unwrap(),
            Seen::LiveUse(older)
        );
    }

    #[test]
    fn decide_after_probe_matches_the_write_rules() {
        let published = parse_version("0.5.0").unwrap();
        let older = Seen::LiveUse(parse_version("0.4.1").unwrap());
        let same = Seen::LiveUse(published);
        let newer = Seen::LiveUse(parse_version("0.5.1").unwrap());
        assert_eq!(
            decide(older, published, false, false),
            Decision::Update {
                from: parse_version("0.4.1").unwrap()
            }
        );
        assert_eq!(decide(same, published, false, false), Decision::UpToDate);
        assert_eq!(decide(newer, published, false, true), Decision::DeviceNewer);
        assert_eq!(
            decide(Seen::Foreign, published, false, true),
            Decision::ConfirmForeign
        );
        assert_eq!(
            decide(Seen::Foreign, published, true, true),
            Decision::ReplaceForeign
        );
        assert_eq!(
            decide(Seen::Bootloader, published, false, false),
            Decision::LeaveBootloader
        );
        assert_eq!(
            decide(Seen::Bootloader, published, false, true),
            Decision::FirstSetup
        );
        assert_eq!(
            decide(Seen::Absent, published, false, true),
            Decision::PhysicalSteps
        );
        assert_eq!(
            decide(Seen::Absent, published, false, false),
            Decision::NotConnected
        );
    }
}
