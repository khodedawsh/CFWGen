use chrono::Utc;
use serde_json::{json, Value};

use js_sys::{Array, Math, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub fn generate_random_string(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789";
    (0..len)
        .map(|_| {
            let idx = (Math::random() * (CHARSET.len() as f64)) as usize;
            CHARSET[idx] as char
        })
        .collect()
}

pub(crate) async fn generate_warp_request_body(public_key: &str) -> Value {
    let install_id = generate_random_string(22);

    let body = json!({
        "key": public_key,
        "install_id": install_id,
        "fcm_token": format!("{}:APA91b{}", install_id, generate_random_string(134)),
        "tos": Utc::now().to_rfc3339(),
        "type": "Android",
        "locale": "en_US"
    });
    body
}

fn crypto() -> web_sys::Crypto {
    if let Some(w) = web_sys::window() {
        w.crypto().expect("crypto")
    } else {
        let g = js_sys::global();
        let scope: web_sys::WorkerGlobalScope = g.unchecked_into();
        scope.crypto().expect("crypto")
    }
}

fn to_std_base64(b64url: &str) -> String {
    b64url.replace('-', "+").replace('_', "/") + "="
}

pub async fn gen_x25519() -> Result<(String, String), JsValue> {
    let subtle = crypto().subtle();

    let alg = Object::new();
    Reflect::set(&alg, &"name".into(), &"X25519".into())?;

    let usages = Array::new();
    usages.push(&"deriveBits".into());
    let promise = subtle.generate_key_with_object(&alg, true, &usages)?;

    let key_pair = JsFuture::from(promise).await?;
    let private_key = Reflect::get(&key_pair, &"privateKey".into())?;
    let public_key = Reflect::get(&key_pair, &"publicKey".into())?;

    let pub_jwk_val =
        JsFuture::from(subtle.export_key("jwk", &public_key.unchecked_into())?).await?;
    let x_b64url = Reflect::get(&pub_jwk_val, &"x".into())?
        .as_string()
        .ok_or_else(|| JsValue::from_str("missing 'x' in JWK"))?;
    let pub_b64 = x_b64url;

    let pk_jwk_val =
        JsFuture::from(subtle.export_key("jwk", &private_key.unchecked_into())?).await?;
    let pk_b64 = Reflect::get(&pk_jwk_val, &"d".into())?
        .as_string()
        .ok_or_else(|| JsValue::from_str("missing 'd' in JWK"))?;

    let pk_b64 = to_std_base64(&pk_b64);
    let pub_b64 = to_std_base64(&pub_b64);

    web_sys::console::log_1(&format!("Public (b64): {}", pub_b64).into());
    web_sys::console::log_1(&format!("Private (b64): {}", pk_b64).into());

    Ok((pub_b64, pk_b64))
}

fn generate_config(
    account_private_key: &str,
    addresses_with_cidr: &str,
    dns: &str,
    account_peer_public_key: &str,
    endpoint: &str,
) -> String {
    format!(
        "[Interface]\n\
        PrivateKey = {private_key}\n\
        Address = {addresses}\n\
        DNS = {dns}\n\n\
        [Peer]\n\
        PublicKey = {peer_key}\n\
        AllowedIPs = 0.0.0.0/0, ::/0\n\
        Endpoint = {endpoint}",
        private_key = account_private_key,
        addresses = addresses_with_cidr,
        dns = dns,
        peer_key = account_peer_public_key,
        endpoint = endpoint
    )
}

pub(crate) fn generate_config_from_account(
    data: Value,
    private_key: &str,
    endpoint: &str,
) -> String {
    let config = data
        .as_object()
        .and_then(|o| o.get("config"))
        .and_then(|c| c.as_object())
        .unwrap();
    let peer = config
        .get("peers")
        .and_then(|p| p.as_array())
        .and_then(|a| a.first())
        .and_then(|p| p.as_object())
        .unwrap();
    let peer_public_key = peer.get("public_key").and_then(|pk| pk.as_str()).unwrap();
    let addresses = config
        .get("interface")
        .and_then(|i| i.get("addresses"))
        .and_then(|a| a.as_object())
        .unwrap();
    let v4 = addresses.get("v4").and_then(|v4| v4.as_str()).unwrap();
    let v6 = addresses.get("v6").and_then(|v6| v6.as_str()).unwrap();

    let wg_addresses = format!("{v4}/32, {v6}/128");
    generate_config(
        private_key,
        &wg_addresses,
        "1.1.1.1, 8.8.8.8",
        peer_public_key,
        endpoint,
    )
}
