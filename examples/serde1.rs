use std::{fmt, str::FromStr};

use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chacha20poly1305::{
    aead::{Aead, OsRng},
    AeadCore, ChaCha20Poly1305, KeyInit,
};
use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
const KEY: &[u8] = b"0123456789abcdef0123456789abcdef";
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct User {
    name: String,
    age: u8,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    skills: Vec<String>,
    state: WorkState,
    #[serde(serialize_with = "b64_encode", deserialize_with = "b64_decode")]
    data: Vec<u8>,
    // #[serde(
    //     serialize_with = "serialize_encrypt",
    //     deserialize_with = "deserialize_decrypt"
    // )]
    #[serde_as(as = "DisplayFromStr")]
    sensitive: SensitiveData,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    url: Vec<http::Uri>,
}

#[derive(Debug)]
struct SensitiveData(String);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
enum WorkState {
    Working(String),
    OnLeave(DateTime<Utc>),
    Terminated,
}

fn main() -> anyhow::Result<()> {
    // let state = WorkState::Working("Rust engineer".to_string());
    let state1 = WorkState::OnLeave(Utc::now());
    let user = User {
        name: "John".to_string(),
        age: 30,
        skills: vec![],
        state: state1,
        data: vec![1, 2, 3, 4, 5],
        sensitive: SensitiveData::new("secret"),
        url: vec!["https://example.com".parse()?],
    };
    let json = serde_json::to_string(&user)?;
    println!("json :{}", json);
    let user1: User = serde_json::from_str(&json)?;
    println!("user1 :{:?}", user1);

    Ok(())
}
fn b64_encode<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let encoded = URL_SAFE_NO_PAD.encode(data);
    serializer.serialize_str(&encoded)
}

fn b64_decode<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded.as_bytes())
        .map_err(serde::de::Error::custom)?;
    Ok(decoded)
}
#[allow(dead_code)]
fn serialize_encrypt<S>(data: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let encrypted = encrypt(data.as_bytes()).unwrap();
    serializer.serialize_str(&encrypted)
}

#[allow(dead_code)]
fn deserialize_decrypt<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let encrypted = String::deserialize(deserializer)?;
    let decrypted = decrypt(&encrypted).unwrap();
    Ok(decrypted)
}

fn encrypt(data: &[u8]) -> anyhow::Result<String> {
    let cipher = chacha20poly1305::ChaCha20Poly1305::new(KEY.into());
    let nonce = ChaCha20Poly1305::generate_nonce(OsRng);
    let encrypted = cipher.encrypt(&nonce, data).unwrap();
    let nonce_ciphertext: Vec<_> = nonce.iter().copied().chain(encrypted).collect();

    let encoded = URL_SAFE_NO_PAD.encode(nonce_ciphertext);
    Ok(encoded)
}

fn decrypt(data: &str) -> anyhow::Result<String> {
    let decoded = URL_SAFE_NO_PAD.decode(data.as_bytes())?;
    let cipher = chacha20poly1305::ChaCha20Poly1305::new(KEY.into());
    // let nonce = ChaCha20Poly1305::generate_nonce(OsRng);
    let nonce = decoded[..12].into();
    let decrypted = cipher.decrypt(nonce, &decoded[12..]).unwrap();
    Ok(String::from_utf8(decrypted)?)
}

impl fmt::Display for SensitiveData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let encrypted = encrypt(self.0.as_bytes()).unwrap();
        write!(f, "{}", encrypted)
    }
}

impl FromStr for SensitiveData {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decrypted = decrypt(s)?;
        Ok(SensitiveData(decrypted))
    }
}

impl SensitiveData {
    fn new(data: &str) -> Self {
        Self(data.to_string())
    }
}
