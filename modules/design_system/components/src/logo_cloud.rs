use maud::{html, Markup};

/// Represents a logo in the logo cloud component.
#[derive(Debug, Clone)]
pub struct Logo {
    pub src: String,
    pub alt: String,
    pub width: u32,
    pub height: u32,
}

impl Logo {
    /// Creates a new Logo instance.
    ///
    /// # Arguments
    ///
    /// * `src` - The source URL of the logo image
    /// * `alt` - The alt text for the logo
    /// * `width` - The width attribute for the image
    /// * `height` - The height attribute for the image
    pub fn new(src: impl Into<String>, alt: impl Into<String>, width: u32, height: u32) -> Self {
        Self { src: src.into(), alt: alt.into(), width, height }
    }
}

/// Renders a logo cloud component displaying a collection of logos in a responsive grid.
///
/// # Arguments
///
/// * `title` - The title text displayed above the logo grid
/// * `logos` - A vector of Logo instances to display
///
/// # Example
///
/// ```
/// use components::logo_cloud::{logo_cloud, Logo};
///
/// let logos = vec![
///     Logo::new("https://example.com/logo1.svg", "Company 1", 158, 48),
///     Logo::new("https://example.com/logo2.svg", "Company 2", 158, 48),
/// ];
///
/// let markup = logo_cloud("Trusted by the world's most innovative teams", logos);
/// ```
pub fn logo_cloud(title: impl Into<String>, logos: Vec<Logo>) -> Markup {
    let title_text = title.into();

    html! {
        div class="bg-white py-24 sm:py-32" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                h2 class="text-center text-lg/8 font-semibold text-gray-900" {
                    (title_text)
                }
                div class="mx-auto mt-10 grid max-w-lg grid-cols-4 items-center gap-x-8 gap-y-10 sm:max-w-xl sm:grid-cols-6 sm:gap-x-10 lg:mx-0 lg:max-w-none lg:grid-cols-5" {
                    @for (index, logo) in logos.iter().enumerate() {
                        img
                            width=(logo.width)
                            height=(logo.height)
                            src=(logo.src)
                            alt=(logo.alt)
                            class={
                                @if logos.len() == 5 {
                                    // Special positioning for 5 logos to match Tailwind reference
                                    @if index == 3 {
                                        "col-span-2 max-h-12 w-full object-contain sm:col-start-2 lg:col-span-1"
                                    } @else if index == 4 {
                                        "col-span-2 col-start-2 max-h-12 w-full object-contain sm:col-start-auto lg:col-span-1"
                                    } @else {
                                        "col-span-2 max-h-12 w-full object-contain lg:col-span-1"
                                    }
                                } @else {
                                    // Default responsive grid for other counts
                                    "col-span-2 max-h-12 w-full object-contain lg:col-span-1"
                                }
                            };
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logo_creation() {
        let logo = Logo::new("https://example.com/logo.svg", "Test Logo", 158, 48);

        assert_eq!(logo.src, "https://example.com/logo.svg");
        assert_eq!(logo.alt, "Test Logo");
        assert_eq!(logo.width, 158);
        assert_eq!(logo.height, 48);
    }

    #[test]
    fn test_logo_clone() {
        let logo = Logo::new("https://example.com/logo.svg", "Test Logo", 158, 48);
        let cloned = logo.clone();

        assert_eq!(logo.src, cloned.src);
        assert_eq!(logo.alt, cloned.alt);
        assert_eq!(logo.width, cloned.width);
        assert_eq!(logo.height, cloned.height);
    }

    #[test]
    fn test_rendered_html_contains_title() {
        let logos = vec![Logo::new("https://example.com/logo.svg", "Test", 158, 48)];
        let markup = logo_cloud("Trusted Partners", logos);
        let html = markup.into_string();

        assert!(html.contains("Trusted Partners"));
        assert!(html.contains("<h2"));
        assert!(html.contains("text-center"));
    }

    #[test]
    fn test_logos_vector_generates_correct_img_elements() {
        let logos = vec![
            Logo::new("https://example.com/logo1.svg", "Logo 1", 158, 48),
            Logo::new("https://example.com/logo2.svg", "Logo 2", 158, 48),
            Logo::new("https://example.com/logo3.svg", "Logo 3", 158, 48),
        ];
        let markup = logo_cloud("Test Title", logos);
        let html = markup.into_string();

        // Check that all three logos are present
        assert!(html.contains("logo1.svg"));
        assert!(html.contains("logo2.svg"));
        assert!(html.contains("logo3.svg"));

        // Count img tags
        let img_count = html.matches("<img").count();
        assert_eq!(img_count, 3, "Should render exactly 3 img elements");
    }

    #[test]
    fn test_logo_attributes_rendered_correctly() {
        let logos = vec![Logo::new("https://example.com/test-logo.svg", "Test Company", 200, 60)];
        let markup = logo_cloud("Test", logos);
        let html = markup.into_string();

        assert!(html.contains("src=\"https://example.com/test-logo.svg\""));
        assert!(html.contains("alt=\"Test Company\""));
        assert!(html.contains("width=\"200\""));
        assert!(html.contains("height=\"60\""));
    }

    #[test]
    fn test_responsive_grid_classes_present() {
        let logos = vec![Logo::new("https://example.com/logo.svg", "Test", 158, 48)];
        let markup = logo_cloud("Test", logos);
        let html = markup.into_string();

        // Check for responsive grid classes
        assert!(html.contains("grid"));
        assert!(html.contains("grid-cols-4"));
        assert!(html.contains("sm:grid-cols-6"));
        assert!(html.contains("lg:grid-cols-5"));
        assert!(html.contains("gap-x-8"));
        assert!(html.contains("gap-y-10"));
    }

    #[test]
    fn test_empty_logos_vector_handling() {
        let logos: Vec<Logo> = vec![];
        let markup = logo_cloud("No Logos", logos);
        let html = markup.into_string();

        // Should still render the structure with title
        assert!(html.contains("No Logos"));
        assert!(html.contains("<h2"));

        // Should not contain any img tags
        assert!(!html.contains("<img"));
    }

    #[test]
    fn test_single_logo_rendering() {
        let logos = vec![Logo::new("https://example.com/single.svg", "Single Logo", 158, 48)];
        let markup = logo_cloud("Single Logo Test", logos);
        let html = markup.into_string();

        assert!(html.contains("Single Logo Test"));
        assert!(html.contains("single.svg"));

        let img_count = html.matches("<img").count();
        assert_eq!(img_count, 1);
    }

    #[test]
    fn test_five_logos_special_positioning() {
        let logos = vec![
            Logo::new("https://example.com/logo1.svg", "Logo 1", 158, 48),
            Logo::new("https://example.com/logo2.svg", "Logo 2", 158, 48),
            Logo::new("https://example.com/logo3.svg", "Logo 3", 158, 48),
            Logo::new("https://example.com/logo4.svg", "Logo 4", 158, 48),
            Logo::new("https://example.com/logo5.svg", "Logo 5", 158, 48),
        ];
        let markup = logo_cloud("Five Logos", logos);
        let html = markup.into_string();

        // Fourth logo (index 3) should have special positioning
        assert!(html.contains("sm:col-start-2"));

        // Fifth logo (index 4) should have different positioning
        assert!(html.contains("col-start-2"));
        assert!(html.contains("sm:col-start-auto"));
    }

    #[test]
    fn test_ten_logos_rendering() {
        let logos = (0..10)
            .map(|i| Logo::new(format!("https://example.com/logo{}.svg", i), format!("Logo {}", i), 158, 48))
            .collect();

        let markup = logo_cloud("Ten Logos", logos);
        let html = markup.into_string();

        let img_count = html.matches("<img").count();
        assert_eq!(img_count, 10);
    }

    #[test]
    fn test_component_structure_matches_reference() {
        let logos = vec![Logo::new("https://example.com/test.svg", "Test", 158, 48)];
        let markup = logo_cloud("Test", logos);
        let html = markup.into_string();

        // Verify outer container
        assert!(html.contains("bg-white"));
        assert!(html.contains("py-24"));
        assert!(html.contains("sm:py-32"));

        // Verify inner container
        assert!(html.contains("mx-auto"));
        assert!(html.contains("max-w-7xl"));
        assert!(html.contains("px-6"));
        assert!(html.contains("lg:px-8"));

        // Verify title styling
        assert!(html.contains("text-center"));
        assert!(html.contains("text-lg/8"));
        assert!(html.contains("font-semibold"));
        assert!(html.contains("text-gray-900"));

        // Verify grid container
        assert!(html.contains("mt-10"));
        assert!(html.contains("max-w-lg"));
        assert!(html.contains("sm:max-w-xl"));
        assert!(html.contains("lg:max-w-none"));
    }

    #[test]
    fn test_image_styling_classes() {
        let logos = vec![Logo::new("https://example.com/test.svg", "Test", 158, 48)];
        let markup = logo_cloud("Test", logos);
        let html = markup.into_string();

        // Each image should have these classes
        assert!(html.contains("max-h-12"));
        assert!(html.contains("w-full"));
        assert!(html.contains("object-contain"));
    }
}
