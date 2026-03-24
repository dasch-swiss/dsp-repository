use leptos::prelude::*;
use mosaic_tiles::icon::{Document, Download, Icon, IconGitHub, IconLinkedIn, IconX};

fn current_year() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    1970 + secs / 31_557_600
}

#[component]
pub fn Footer() -> impl IntoView {
    let year = current_year();

    view! {
        <footer class="bg-slate-800 text-gray-300 py-12">
            <div class="dpe-max-layout-width mx-auto px-4">
                <nav class="flex flex-wrap justify-center gap-6 mb-8">
                    <a
                        class="hover:text-white transition-colors"
                        href="https://dasch.swiss/legal-notice"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "Legal Notice"
                    </a>
                    <a
                        class="hover:text-white transition-colors"
                        href="https://dasch.swiss/privacy-policy"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "Privacy Policy"
                    </a>
                    <a
                        class="hover:text-white transition-colors"
                        href="https://dasch.swiss/privacy-policy-en"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "Privacy Policy (English)"
                    </a>
                    <a
                        class="hover:text-white transition-colors"
                        href="https://dasch.swiss/impressum"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "Impressum"
                    </a>
                    <a
                        class="hover:text-white transition-colors"
                        href="https://dasch.swiss/contact"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "Contact"
                    </a>
                </nav>

                <hr class="border-slate-700 mb-8" />

                <div class="text-center mb-8">
                    <p class="tracking-wider uppercase text-white mb-4">"Downloads"</p>
                    <nav class="flex flex-wrap justify-center gap-8 text-sm">
                        <a
                            class="hover:text-white transition-colors flex items-center gap-1 px-4"
                            href="https://dasch.swiss/downloads/AGB_DaSCH_4.0.pdf"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            <Icon icon=Download class="w-4 h-4" />
                            "Terms and Conditions (AGB)"
                        </a>
                        <a
                            class="hover:text-white transition-colors flex items-center gap-1 px-4"
                            href="https://dasch.swiss/downloads/DaSCH_Deposit_Agreement.pdf"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            <Icon icon=Download class="w-4 h-4" />
                            "Deposit Agreement"
                        </a>
                        <a
                            class="hover:text-white transition-colors flex items-center gap-1 px-4"
                            href="https://dasch.swiss/downloads/20220214_DaSCH_Statuten_Version_2022_def.pdf"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            <Icon icon=Download class="w-4 h-4" />
                            "DaSCH Statutes 2022"
                        </a>
                        <a
                            class="hover:text-white transition-colors flex items-center gap-1 px-4"
                            href="https://dasch.swiss/downloads/ToS_NB_V07.pdf"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            <Icon icon=Download class="w-4 h-4" />
                            "Terms of Service"
                        </a>
                    </nav>
                </div>

                <div class="flex justify-center gap-6 mb-8">
                    <a
                        href="https://www.linkedin.com/company/dasch-swiss"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-white hover:text-white transition-colors"
                        aria-label="DaSCH on LinkedIn"
                    >
                        <Icon icon=IconLinkedIn class="w-6 h-6" />
                    </a>
                    <a
                        href="https://x.com/daschswiss"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-white hover:text-white transition-colors"
                        aria-label="DaSCH on X (Twitter)"
                    >
                        <Icon icon=IconX class="w-6 h-6" />
                    </a>
                    <a
                        href="https://github.com/dasch-swiss"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-white hover:text-white transition-colors"
                        aria-label="DaSCH on GitHub"
                    >
                        <Icon icon=IconGitHub class="w-6 h-6" />
                    </a>
                </div>

                <div class="text-center text-sm text-gray-400">
                    <p>
                        {format!(
                            "© {year} DaSCH \u{2013} Swiss National Data and Service Center for the Humanities. All rights reserved.",
                        )}
                    </p>
                </div>
            </div>
        </footer>
    }
}
