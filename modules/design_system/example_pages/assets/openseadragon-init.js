// Initialize OpenSeadragon viewer for Column 3
document.addEventListener('DOMContentLoaded', function() {
    const viewerElement = document.getElementById('openseadragon-viewer');

    if (viewerElement) {
        OpenSeadragon({
            id: "openseadragon-viewer",
            prefixUrl: "https://cdnjs.cloudflare.com/ajax/libs/openseadragon/5.0.1/images/",
            tileSources: {
                type: 'image',
                url: 'https://openseadragon.github.io/example-images/grand-canyon-landscape-overlooking.jpg'
            },
            showNavigationControl: true,
            showFullPageControl: true,
            showHomeControl: true,
            showZoomControl: true
        });
    }
});
