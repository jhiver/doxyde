// Mobile navigation management
(function() {
    let currentOpenMenu = null;
    let touchStartX = null;
    
    function openMenu(menuType) {
        // Close any open menu first
        if (currentOpenMenu && currentOpenMenu !== menuType) {
            closeMenu(currentOpenMenu);
        }
        
        const drawer = document.querySelector(`.mobile-${menuType}-drawer`);
        const overlay = document.querySelector('.mobile-menu-overlay');
        
        if (!drawer || !overlay) return;
        
        drawer.classList.add('open');
        overlay.classList.add('show');
        document.body.classList.add('mobile-menu-open');
        
        currentOpenMenu = menuType;
        
        // Focus management
        drawer.setAttribute('tabindex', '-1');
        drawer.focus();
    }
    
    function closeMenu(menuType) {
        if (!menuType) menuType = currentOpenMenu;
        if (!menuType) return;
        
        const drawer = document.querySelector(`.mobile-${menuType}-drawer`);
        const overlay = document.querySelector('.mobile-menu-overlay');
        
        if (!drawer || !overlay) return;
        
        drawer.classList.remove('open');
        overlay.classList.remove('show');
        document.body.classList.remove('mobile-menu-open');
        
        currentOpenMenu = null;
        
        // Return focus to the toggle button
        let toggleButton;
        if (menuType === 'nav') {
            toggleButton = document.querySelector('.mobile-nav-toggle');
        } else if (menuType === 'edit') {
            toggleButton = document.querySelector('.mobile-edit-toggle');
        } else if (menuType === 'controls') {
            toggleButton = document.querySelector('.mobile-controls-toggle');
        }
        if (toggleButton) toggleButton.focus();
    }
    
    function toggleMenu(menuType) {
        if (currentOpenMenu === menuType) {
            closeMenu(menuType);
        } else {
            openMenu(menuType);
        }
    }
    
    // Initialize event listeners when DOM is ready
    function init() {
        // Navigation toggle
        const navToggle = document.querySelector('.mobile-nav-toggle');
        if (navToggle) {
            navToggle.addEventListener('click', () => toggleMenu('nav'));
        }
        
        // Edit toggle
        const editToggle = document.querySelector('.mobile-edit-toggle');
        if (editToggle) {
            editToggle.addEventListener('click', () => toggleMenu('edit'));
        }
        
        // Controls toggle (save/draft controls)
        const controlsToggle = document.querySelector('.mobile-controls-toggle');
        if (controlsToggle) {
            controlsToggle.addEventListener('click', () => toggleMenu('controls'));
        }
        
        // Close buttons
        document.querySelectorAll('.mobile-drawer-close').forEach(button => {
            button.addEventListener('click', () => closeMenu());
        });
        
        // Overlay click
        const overlay = document.querySelector('.mobile-menu-overlay');
        if (overlay) {
            overlay.addEventListener('click', () => closeMenu());
        }
        
        // Escape key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && currentOpenMenu) {
                closeMenu();
            }
        });
        
        // Touch gestures for closing
        document.addEventListener('touchstart', (e) => {
            if (currentOpenMenu && e.touches.length === 1) {
                touchStartX = e.touches[0].clientX;
            }
        });
        
        document.addEventListener('touchmove', (e) => {
            if (!currentOpenMenu || !touchStartX || e.touches.length !== 1) return;
            
            const touchEndX = e.touches[0].clientX;
            const drawer = document.querySelector(`.mobile-${currentOpenMenu}-drawer`);
            
            if (!drawer) return;
            
            // Swipe right to close (drawers open from right)
            const diff = touchEndX - touchStartX;
            if (diff > 50) {
                closeMenu();
                touchStartX = null;
            }
        });
        
        document.addEventListener('touchend', () => {
            touchStartX = null;
        });
        
        // Close menus when clicking on navigation links
        document.querySelectorAll('.mobile-nav-item, .mobile-edit-item').forEach(link => {
            if (link.tagName === 'A') {
                link.addEventListener('click', () => {
                    closeMenu();
                });
            }
        });
    }
    
    // Initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();