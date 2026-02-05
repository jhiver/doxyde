/**
 * Gallery Modal - Fullscreen image gallery with keyboard and touch navigation
 */
(function() {
    'use strict';

    let images = [];
    let currentIndex = 0;
    let modal = null;
    let img = null;
    let counter = null;
    let touchStartX = 0;

    function init() {
        modal = document.getElementById('gallery-modal');
        if (!modal) {
            return false;
        }
        img = document.getElementById('gallery-image');
        counter = document.querySelector('.gallery-counter');

        // Close on backdrop click
        modal.addEventListener('click', function(e) {
            if (e.target === modal) {
                close();
            }
        });

        // Keyboard navigation
        document.addEventListener('keydown', handleKeydown);

        // Touch swipe support
        modal.addEventListener('touchstart', function(e) {
            touchStartX = e.touches[0].clientX;
        }, { passive: true });

        modal.addEventListener('touchend', function(e) {
            var diff = touchStartX - e.changedTouches[0].clientX;
            if (Math.abs(diff) > 50) {
                if (diff > 0) {
                    next();
                } else {
                    prev();
                }
            }
        }, { passive: true });

        return true;
    }

    function open(imgs, startIndex) {
        if (!imgs || imgs.length === 0) {
            return;
        }
        images = imgs;
        currentIndex = startIndex || 0;

        if (!modal && !init()) {
            return;
        }

        show(currentIndex);
        modal.classList.add('active');
        document.body.style.overflow = 'hidden';
    }

    function close() {
        if (!modal) {
            return;
        }
        modal.classList.remove('active');
        document.body.style.overflow = '';
    }

    function show(index) {
        if (!img || !counter || images.length === 0) {
            return;
        }
        currentIndex = index;
        img.src = images[index].src;
        img.alt = images[index].alt || '';
        counter.textContent = (index + 1) + ' / ' + images.length;
    }

    function next() {
        if (images.length === 0) {
            return;
        }
        show((currentIndex + 1) % images.length);
    }

    function prev() {
        if (images.length === 0) {
            return;
        }
        show((currentIndex - 1 + images.length) % images.length);
    }

    function handleKeydown(e) {
        if (!modal || !modal.classList.contains('active')) {
            return;
        }
        if (e.key === 'Escape') {
            close();
        } else if (e.key === 'ArrowRight') {
            next();
        } else if (e.key === 'ArrowLeft') {
            prev();
        }
    }

    // Expose functions globally
    window.openGallery = open;
    window.closeGallery = close;
    window.galleryNext = next;
    window.galleryPrev = prev;
})();
