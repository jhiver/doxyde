// Clipboard functionality for code blocks
(function() {
    'use strict';

    // Function to create copy button
    function createCopyButton() {
        const button = document.createElement('button');
        button.className = 'copy-button';
        button.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
        button.setAttribute('aria-label', 'Copy code to clipboard');
        button.setAttribute('title', 'Copy');
        return button;
    }

    // Function to copy text to clipboard
    async function copyToClipboard(text, button) {
        try {
            await navigator.clipboard.writeText(text);
            
            // Show success state
            button.classList.add('copy-success');
            button.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>';
            button.setAttribute('title', 'Copied!');
            
            // Reset button after 2 seconds
            setTimeout(() => {
                button.classList.remove('copy-success');
                button.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
                button.setAttribute('title', 'Copy');
            }, 2000);
        } catch (err) {
            console.error('Failed to copy text: ', err);
            
            // Show error state
            button.classList.add('copy-error');
            button.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>';
            button.setAttribute('title', 'Failed to copy');
            
            setTimeout(() => {
                button.classList.remove('copy-error');
                button.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
                button.setAttribute('title', 'Copy');
            }, 2000);
        }
    }

    // Initialize copy buttons for all code blocks
    function initializeCodeBlocks() {
        // Find all pre elements containing code
        const codeBlocks = document.querySelectorAll('pre > code');
        
        codeBlocks.forEach(codeElement => {
            const preElement = codeElement.parentElement;
            
            // Skip if already has a copy button
            if (preElement.querySelector('.copy-button')) {
                return;
            }
            
            // Create wrapper div if not already wrapped
            if (!preElement.parentElement.classList.contains('code-block-wrapper')) {
                const wrapper = document.createElement('div');
                wrapper.className = 'code-block-wrapper';
                preElement.parentNode.insertBefore(wrapper, preElement);
                wrapper.appendChild(preElement);
            }
            
            // Add copy button
            const copyButton = createCopyButton();
            copyButton.addEventListener('click', () => {
                const codeText = codeElement.textContent || codeElement.innerText;
                copyToClipboard(codeText, copyButton);
            });
            
            preElement.appendChild(copyButton);
        });
    }

    // Initialize on DOM content loaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initializeCodeBlocks);
    } else {
        initializeCodeBlocks();
    }

    // Re-initialize when new content is added (for dynamic content)
    if (typeof MutationObserver !== 'undefined') {
        const observer = new MutationObserver(() => {
            initializeCodeBlocks();
        });
        
        observer.observe(document.body, {
            childList: true,
            subtree: true
        });
    }
})();