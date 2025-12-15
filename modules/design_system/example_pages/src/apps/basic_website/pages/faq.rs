use components::{hero, ComponentBuilder};
use maud::{html, Markup};

use crate::layout::page_layout;

/// FAQ page
pub async fn faq() -> Markup {
    let content = html! {
        (hero::hero("Frequently Asked Questions")
            .with_description("Find answers to common questions about our services and platform.")
            .with_id("faq-heading")
            .build())

        section class="bg-white py-24 sm:py-32 dark:bg-gray-900" aria-labelledby="faq-heading" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                // FAQ content
                div class="mx-auto mt-16 max-w-4xl" {
                    div class="space-y-12" {
                        // Category 1: Getting Started
                        div class="border-b border-gray-200 pb-8 dark:border-gray-700" {
                            h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "Getting Started" }

                            div class="mt-6 space-y-6" {
                                // Q1
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Is DaSCH suitable for my project?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "DaSCH is ideal for qualitative humanities research involving text, images, audio, or video. Contact us to assess your project's compatibility with our infrastructure." }
                                    }
                                }

                                // Q2
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Can you help me write a Data Management Plan?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Yes. We offer free workshops and consulting on DMP writing, including guidance on FAIR principles and best practices for qualitative humanities data." }
                                    }
                                }

                                // Q3
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "What counts as research data?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Research data includes any information collected, observed, or created during research. For humanities, this typically includes texts, manuscripts, images, audio/video recordings, transcriptions, annotations, and metadata." }
                                    }
                                }
                            }
                        }

                        // Category 2: Data Types & Formats
                        div class="border-b border-gray-200 pb-8 dark:border-gray-700" {
                            h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "Data Types & Formats" }

                            div class="mt-6 space-y-6" {
                                // Q4
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "What types of data can I archive?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "DaSCH supports structured data, files, and multimedia content including texts, images, audio recordings, and video materials commonly used in humanities research." }
                                    }
                                }

                                // Q5
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Which file formats are supported?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "We support standard formats including:" }
                                        ul class="mt-2 list-disc space-y-1 pl-6" {
                                            li { "Documents: PDF, TXT, XML, TEI" }
                                            li { "Images: JPEG, PNG, TIFF" }
                                            li { "Audio/Video: MP3, WAV, MP4, MOV" }
                                            li { "Structured data: CSV, JSON, RDF" }
                                        }
                                    }
                                }
                            }
                        }

                        // Category 3: FAIR Principles & Access
                        div class="border-b border-gray-200 pb-8 dark:border-gray-700" {
                            h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "FAIR Principles & Access" }

                            div class="mt-6 space-y-6" {
                                // Q6
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "How does DaSCH ensure FAIR compliance?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "All archived data receives persistent identifiers, standardized metadata, API access for interoperability, and clear licensing for reuse. Our infrastructure is designed specifically for FAIR principles." }
                                    }
                                }

                                // Q7
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Will my data be publicly available?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Archived data is citable with persistent identifiers and can be made publicly accessible. You control access levels and visibility settings." }
                                    }
                                }

                                // Q8
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Can I restrict access to my data?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Yes. We support various access restrictions including embargoes, restricted access for specific users, and private datasets. You maintain control over who can view and use your data." }
                                    }
                                }
                            }
                        }

                        // Category 4: Platform Features
                        div class="border-b border-gray-200 pb-8 dark:border-gray-700" {
                            h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "Platform Features" }

                            div class="mt-6 space-y-6" {
                                // Q9
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Does DaSCH use international metadata standards?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Yes. We comply with international metadata standards to ensure your data is discoverable and interoperable with other research infrastructures." }
                                    }
                                }

                                // Q10
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "How is my data displayed and accessed?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Data is accessible via DSP-APP (web interface) and our API for programmatic access. Both methods support searching, browsing, and citing your research data." }
                                    }
                                }

                                // Q11
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Can I edit data after archiving?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Yes. DaSCH provides ongoing editing capabilities with version control, allowing you to update and maintain your archived data over time." }
                                    }
                                }

                                // Q12
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "How do users discover my archived data?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Through our search interface, persistent identifiers, metadata catalogs, and integration with international research infrastructure networks." }
                                    }
                                }
                            }
                        }

                        // Category 5: Costs & Support
                        div class="border-b border-gray-200 pb-8 dark:border-gray-700" {
                            h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "Costs & Support" }

                            div class="mt-6 space-y-6" {
                                // Q13
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "What are the costs for archiving?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Repository services are free for Swiss national research projects. Data exceeding 500 GB may require cost-sharing after 2025. See our Services page for detailed pricing." }
                                    }
                                }

                                // Q14
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "What support is available during my project?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "We offer free workshops, up to 8 hours of complimentary consulting, custom script development, and ongoing technical support throughout your project." }
                                    }
                                }
                            }
                        }

                        // Category 6: Technical Requirements
                        div class="pb-8" {
                            h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "Technical Requirements" }

                            div class="mt-6 space-y-6" {
                                // Q15
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Do I need technical or IT knowledge?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Basic computer skills are sufficient for most tasks. We provide training and hands-on support. Our team handles technical implementation details." }
                                    }
                                }

                                // Q16
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "Can I archive sensitive data?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Sensitive data requires special handling. Contact us to discuss anonymization, access restrictions, and compliance requirements for your specific data." }
                                    }
                                }

                                // Q17
                                details class="group" {
                                    summary class="flex cursor-pointer items-start justify-between text-lg font-semibold text-gray-900 dark:text-white" {
                                        span { "How complex is creating a data model?" }
                                        span class="ml-6 flex h-7 items-center" { "▼" }
                                    }
                                    div class="mt-4 text-gray-600 dark:text-gray-400" {
                                        p { "Data model complexity varies by project. Our team provides guidance and consulting to design appropriate models for your research data structure and requirements." }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    page_layout("FAQ - DaSCH Swiss", content)
}
