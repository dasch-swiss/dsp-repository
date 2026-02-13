use leptos::prelude::*;

use crate::components::global::header_links::HeaderLinks;

#[component]
pub fn NavBar() -> impl IntoView {
    view! {
        <div class="navbar bg-base-100 shadow-sm">
            <div class="flex-none">
                <img src="/logo.svg" class="inline h-10 w-10 mr-2" />
            </div>
            <div class="flex-1">
                <a class="btn btn-ghost text-xl" href="/">
                    "DaSCH Metadata Browser"
                </a>
            </div>
            // Mobile dropdown - visible only on small/medium screens
            <div class="flex-none lg:hidden">
                <div class="dropdown dropdown-end">
                    <div tabindex="0" role="button" class="btn btn-ghost btn-circle">
                        <svg
                          xmlns="http://www.w3.org/2000/svg"
                          class="h-5 w-5"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor">
                          <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M4 6h16M4 12h16M4 18h16" />
                        </svg>
                    </div>
                    <ul
                      tabindex="-1"
                      class="menu menu-sm dropdown-content bg-base-100 rounded-box z-[1] mt-3 w-52 p-2 shadow">
        <HeaderLinks />

                    </ul>
                </div>
            </div>

            // Desktop menu - visible only on large screens
            <div class="flex-none hidden lg:flex">
        <HeaderLinks />
            </div>
        </div>
    }
}
