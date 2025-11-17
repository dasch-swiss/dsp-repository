// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><a href="docs.html">About this Documentation</a></li><li class="chapter-item expanded affix "><li class="part-title">Repo Overview</li><li class="chapter-item expanded "><a href="workflows.html"><strong aria-hidden="true">1.</strong> Workflows and Conventions</a></li><li class="chapter-item expanded "><a href="repo_structure.html"><strong aria-hidden="true">2.</strong> Project Structure and Code Organization</a></li><li class="chapter-item expanded "><div><strong aria-hidden="true">3.</strong> Release, Deployment and Versioning</div></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Modules</li><li class="chapter-item expanded affix "><li class="part-title">DPE</li><li class="chapter-item expanded "><div><strong aria-hidden="true">4.</strong> Discovery and Presentation Environment</div></li><li><ol class="section"><li class="chapter-item expanded "><a href="dpe/project_structure.html"><strong aria-hidden="true">4.1.</strong> Project Structure</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Design System</li><li class="chapter-item expanded "><a href="design_system/overview.html"><strong aria-hidden="true">5.</strong> DSP Design System</a></li><li class="chapter-item expanded "><a href="design_system/component_architecture.html"><strong aria-hidden="true">6.</strong> Component Architecture</a></li><li class="chapter-item expanded "><div><strong aria-hidden="true">7.</strong> Components</div></li><li><ol class="section"><li class="chapter-item expanded "><a href="design_system/components/button.html"><strong aria-hidden="true">7.1.</strong> Button</a></li><li class="chapter-item expanded "><a href="design_system/components/icon.html"><strong aria-hidden="true">7.2.</strong> Icon</a></li><li class="chapter-item expanded "><a href="design_system/components/link.html"><strong aria-hidden="true">7.3.</strong> Link</a></li><li class="chapter-item expanded "><a href="design_system/components/menu.html"><strong aria-hidden="true">7.4.</strong> Menu</a></li><li class="chapter-item expanded "><a href="design_system/components/menu-item.html"><strong aria-hidden="true">7.5.</strong> Menu Item</a></li><li class="chapter-item expanded "><a href="design_system/components/shell.html"><strong aria-hidden="true">7.6.</strong> Shell</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Web Components</li><li class="chapter-item expanded "><a href="web_components/overview.html"><strong aria-hidden="true">8.</strong> Web Components Overview</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="web_components/attribution_badge.html"><strong aria-hidden="true">8.1.</strong> Attribution Badge</a></li></ol></li><li class="chapter-item expanded "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Fundamentals</li><li class="chapter-item expanded "><a href="fundamentals/processes.html"><strong aria-hidden="true">9.</strong> Discovery, Design and Development Process</a></li><li class="chapter-item expanded "><a href="fundamentals/tech_stack.html"><strong aria-hidden="true">10.</strong> Tech Stack</a></li><li class="chapter-item expanded "><a href="fundamentals/testing.html"><strong aria-hidden="true">11.</strong> Testing and Quality Assurance</a></li><li class="chapter-item expanded "><div><strong aria-hidden="true">12.</strong> Observability and Monitoring</div></li><li class="chapter-item expanded "><div><strong aria-hidden="true">13.</strong> Security and Privacy</div></li><li class="chapter-item expanded "><div><strong aria-hidden="true">14.</strong> Archtectural Design Records</div></li><li class="chapter-item expanded "><div><strong aria-hidden="true">15.</strong> Miscellaneous</div></li><li><ol class="section"><li class="chapter-item expanded "><div><strong aria-hidden="true">15.1.</strong> Accessibility</div></li><li class="chapter-item expanded "><div><strong aria-hidden="true">15.2.</strong> Search Engine Optimization</div></li><li class="chapter-item expanded "><div><strong aria-hidden="true">15.3.</strong> Internationalization</div></li></ol></li><li class="chapter-item expanded "><a href="fundamentals/onboarding.html"><strong aria-hidden="true">16.</strong> Onboarding</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
