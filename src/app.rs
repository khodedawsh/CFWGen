use crate::endpoint::get_random_address;
use crate::wg_gen::{gen_x25519, generate_config_from_account, generate_warp_request_body};
use base64::Engine;
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use reqwasm::http::Request;
use yew::platform::spawn_local;
use yew::prelude::*;

fn as_download_data_url(text: &str) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    format!("data:text/plain;charset=utf-8;base64,{b64}")
}

fn gen_qr(text: &str) -> String {
    let code =
        QrCode::with_error_correction_level(text.as_bytes(), EcLevel::Q).expect("valid QR data");

    let svg = code.render::<svg::Color>().min_dimensions(400, 400).build();

    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg.as_bytes())
    )
}

#[function_component(App)]
pub fn app() -> Html {
    let data = use_state(|| None::<String>);
    let qr_src = use_state(|| None::<String>);
    let download_data = use_state(|| None::<String>);
    {
        let data = data.clone();
        let qr_src = qr_src.clone();
        let download_data = download_data.clone();

        use_effect_with((), move |_| {
            // dbg!(rq.build().unwrap().url());
            spawn_local(async move {
                let kp = gen_x25519().await.unwrap();
                let body = &generate_warp_request_body(&kp.0).await.to_string();

                match Request::post(
                    "https://corsproxy.io/?url=https://api.cloudflareclient.com/v0a4005/reg",
                )
                .body(body)
                .send()
                .await
                {
                    Ok(response) => {
                        if let Ok(v) = response.json::<serde_json::Value>().await {
                            let cfg = generate_config_from_account(v, &kp.1, get_random_address());

                            qr_src.set(Some(gen_qr(&cfg)));
                            download_data.set(Some(as_download_data_url(&cfg)));
                            data.set(Some(cfg));
                        }
                    }
                    Err(err) => {
                        data.set(Some(format!("Error: {:?}", err)));
                    }
                }
            });
            || ()
        });
    }

    html! {
        <>
        <div class="container">
        <div style="justify-content:center; align-items:center;">
            <h1 style="display:block; margin:1rem auto; text-align: center;">{ "Warp Config Generator" }</h1>
        {
                match ((*data).as_ref(), (*qr_src).as_ref(), (*download_data).as_ref()) {
                    (Some(content), Some(src), Some(dd)) => html! {
                        <>
                            <img src={src.clone()} style="display:block; margin:1rem auto; max-width:400px;"/>
                            <pre>{ content }</pre>
                            <a
                                    href={ dd.clone() }
                                    download="wireguard.conf"
                                    style="display:block; margin:1rem auto; max-width:400px; text-align:center;"
                                >
                                    <button class="btn btn--lg btn--cold-blue">{ "Download config" }</button>
                                </a>

                        </>
                    },
                    _ => html! { "Loading..." },
                }
            }

        </div>
        </div>
        <footer class="footer">
            <p>
                {"Made by "}
                <a href="https://github.com/khodedawsh" target="_blank" rel="noopener noreferrer">
                    {"Dawsh"}
                </a>
                {" | "}
                <a href="https://github.com/khodedawsh/cfwgen" target="_blank" rel="noopener noreferrer">
                    {"View on GitHub"}
                </a>
            </p>
        </footer>
        </>
    }
}
