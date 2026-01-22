use leptos::prelude::*;
use mosaic_tiles::accordion::{Accordion, AccordionItem};

#[component]
pub fn AccordionAnatomy() -> impl IntoView {
    view! {
        <Accordion>
            <AccordionItem title="Item Title".to_string()>// Item content
            </AccordionItem>
        </Accordion>
    }
}
