use leptos::prelude::*;

const STOPS: &[&str] = &[
    "50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950",
];

const SCALES: &[(&str, &str)] = &[
    ("primary", "#336790"),
    ("secondary", "#74A2CF"),
    ("success", "#31837B"),
    ("danger", "#9E484D"),
    ("warning", "#E39E22"),
    ("info", "#74A2CF"),
    ("accent", "#706DA6"),
    ("neutral", "#3B4856"),
];

#[component]
fn ColorScale(name: &'static str, base_hex: &'static str) -> impl IntoView {
    let swatches = STOPS
        .iter()
        .map(|stop| {
            let var_name = format!("var(--color-{}-{})", name, stop);
            let is_light = matches!(*stop, "50" | "100" | "200" | "300");
            let text_color = if is_light { "#1a1a1a" } else { "#ffffff" };
            view! {
                <div
                    class="flex flex-col items-center justify-end p-2 rounded"
                    style:background-color=var_name
                    style:color=text_color
                    style:min-width="4rem"
                    style:min-height="4rem"
                >
                    <span class="text-xs font-mono">{*stop}</span>
                </div>
            }
        })
        .collect_view();

    view! {
        <div class="mb-6">
            <div class="flex items-baseline gap-2 mb-2">
                <span class="text-sm font-semibold">{name}</span>
                <span class="text-xs text-neutral-500 font-mono">{base_hex}</span>
            </div>
            <div class="flex gap-1">{swatches}</div>
        </div>
    }
}

#[component]
pub fn ColorsExample() -> impl IntoView {
    let scales = SCALES
        .iter()
        .map(|(name, hex)| {
            view! { <ColorScale name=*name base_hex=*hex /> }
        })
        .collect_view();

    view! {
        <div>
            <p class="text-sm text-neutral-600 mb-4">
                "Colors are defined as CSS custom properties (e.g. "
                <code class="text-sm bg-neutral-100 px-1 rounded">"var(--color-primary-500)"</code>
                ") and are also available as Tailwind utilities (e.g. "
                <code class="text-sm bg-neutral-100 px-1 rounded">"bg-primary-500"</code> ")."
            </p>
            {scales}
        </div>
    }
}
