/**
 * Multi-Image Drag and Drop Upload
 *
 * Enables drag-and-drop of multiple images onto the edit page.
 * For each dropped image, creates an image component and uploads the file.
 */

(function() {
    'use strict';

    // Configuration
    const MAX_FILE_SIZE = 10 * 1024 * 1024; // 10MB
    const VALID_TYPES = ['image/jpeg', 'image/png', 'image/gif', 'image/webp', 'image/svg+xml'];
    const MAX_CONCURRENT_UPLOADS = 3;

    // State
    let uploadQueue = [];
    let activeUploads = 0;
    let totalFiles = 0;
    let completedFiles = 0;
    let dragCounter = 0;

    // DOM Elements (created dynamically)
    let overlay = null;
    let queuePanel = null;

    /**
     * Initialize the multi-upload system
     */
    function init() {
        createOverlay();
        createQueuePanel();
        setupGlobalDragDrop();
    }

    /**
     * Create the full-screen drop zone overlay
     */
    function createOverlay() {
        overlay = document.createElement('div');
        overlay.id = 'multi-upload-overlay';
        overlay.innerHTML = `
            <div class="overlay-content">
                <div class="upload-icon">📸</div>
                <h2>Drop images here</h2>
                <p>Release to upload multiple images</p>
            </div>
        `;
        document.body.appendChild(overlay);
    }

    /**
     * Create the upload queue panel
     */
    function createQueuePanel() {
        queuePanel = document.createElement('div');
        queuePanel.id = 'multi-upload-queue';
        queuePanel.innerHTML = `
            <div class="queue-header">
                <span class="queue-title">Uploading...</span>
                <span class="queue-count">0/0</span>
            </div>
            <div class="queue-items"></div>
        `;
        document.body.appendChild(queuePanel);
    }

    /**
     * Set up global drag and drop event listeners
     */
    function setupGlobalDragDrop() {
        document.addEventListener('dragenter', handleDragEnter, false);
        document.addEventListener('dragover', handleDragOver, false);
        document.addEventListener('dragleave', handleDragLeave, false);
        document.addEventListener('drop', handleDrop, false);
    }

    /**
     * Check if the drag event contains image files
     */
    function hasImageFiles(e) {
        if (!e.dataTransfer || !e.dataTransfer.types) {
            return false;
        }

        // Check if dragging files
        if (!e.dataTransfer.types.includes('Files')) {
            return false;
        }

        // Check items for image types (if available)
        if (e.dataTransfer.items) {
            for (let i = 0; i < e.dataTransfer.items.length; i++) {
                const item = e.dataTransfer.items[i];
                if (item.kind === 'file' && item.type.startsWith('image/')) {
                    return true;
                }
            }
            // If no type info available, assume it might be images
            return e.dataTransfer.items.length > 0;
        }

        return true;
    }

    /**
     * Handle drag enter
     */
    function handleDragEnter(e) {
        e.preventDefault();
        e.stopPropagation();

        dragCounter++;

        if (hasImageFiles(e)) {
            showOverlay();
        }
    }

    /**
     * Handle drag over
     */
    function handleDragOver(e) {
        e.preventDefault();
        e.stopPropagation();

        if (hasImageFiles(e)) {
            e.dataTransfer.dropEffect = 'copy';
        }
    }

    /**
     * Handle drag leave
     */
    function handleDragLeave(e) {
        e.preventDefault();
        e.stopPropagation();

        dragCounter--;

        if (dragCounter === 0) {
            hideOverlay();
        }
    }

    /**
     * Handle drop
     */
    function handleDrop(e) {
        e.preventDefault();
        e.stopPropagation();

        dragCounter = 0;
        hideOverlay();

        const files = e.dataTransfer.files;
        if (!files || files.length === 0) {
            return;
        }

        // Filter valid image files
        const imageFiles = [];
        const invalidFiles = [];

        for (let i = 0; i < files.length; i++) {
            const file = files[i];

            if (!VALID_TYPES.includes(file.type)) {
                invalidFiles.push({ name: file.name, reason: 'Invalid type' });
                continue;
            }

            if (file.size > MAX_FILE_SIZE) {
                invalidFiles.push({ name: file.name, reason: 'File too large (max 10MB)' });
                continue;
            }

            imageFiles.push(file);
        }

        // Show warning for invalid files
        if (invalidFiles.length > 0) {
            const messages = invalidFiles.map(f => `${f.name}: ${f.reason}`);
            showNotification('warning', `Some files were skipped:\n${messages.join('\n')}`);
        }

        // Queue valid files for upload
        if (imageFiles.length > 0) {
            queueFiles(imageFiles);
        }
    }

    /**
     * Show the overlay
     */
    function showOverlay() {
        overlay.classList.add('visible');
    }

    /**
     * Hide the overlay
     */
    function hideOverlay() {
        overlay.classList.remove('visible');
    }

    /**
     * Show the queue panel
     */
    function showQueuePanel() {
        queuePanel.classList.add('visible');
    }

    /**
     * Hide the queue panel
     */
    function hideQueuePanel() {
        queuePanel.classList.remove('visible');
    }

    /**
     * Queue files for upload
     */
    function queueFiles(files) {
        totalFiles = files.length;
        completedFiles = 0;

        // Clear previous queue
        uploadQueue = [];
        const queueItems = queuePanel.querySelector('.queue-items');
        queueItems.innerHTML = '';

        // Add files to queue and UI
        for (let i = 0; i < files.length; i++) {
            const file = files[i];
            const item = {
                id: `upload-${Date.now()}-${i}`,
                file: file,
                status: 'pending',
                progress: 0,
                componentId: null
            };

            uploadQueue.push(item);
            addQueueItemUI(item);
        }

        updateQueueCount();
        showQueuePanel();
        processQueue();
    }

    /**
     * Add a queue item to the UI
     */
    function addQueueItemUI(item) {
        const queueItems = queuePanel.querySelector('.queue-items');

        const itemEl = document.createElement('div');
        itemEl.className = 'queue-item';
        itemEl.id = item.id;
        itemEl.innerHTML = `
            <div class="item-name">${escapeHtml(item.file.name)}</div>
            <div class="item-progress">
                <div class="progress-bar" style="width: 0%"></div>
            </div>
            <div class="item-status">Waiting...</div>
        `;

        queueItems.appendChild(itemEl);
    }

    /**
     * Update queue count display
     */
    function updateQueueCount() {
        const countEl = queuePanel.querySelector('.queue-count');
        countEl.textContent = `${completedFiles}/${totalFiles}`;
    }

    /**
     * Process the upload queue
     */
    function processQueue() {
        // Find pending items
        const pendingItems = uploadQueue.filter(item => item.status === 'pending');

        // Start uploads up to max concurrent
        while (activeUploads < MAX_CONCURRENT_UPLOADS && pendingItems.length > 0) {
            const item = pendingItems.shift();
            if (item) {
                uploadItem(item);
            }
        }
    }

    /**
     * Upload a single item
     */
    async function uploadItem(item) {
        item.status = 'uploading';
        activeUploads++;

        updateItemStatus(item.id, 'Creating component...');
        updateItemProgress(item.id, 5);

        try {
            // Step 1: Create image component with metadata from file
            const componentId = await createImageComponent(item.file);
            item.componentId = componentId;

            updateItemStatus(item.id, 'Uploading image...');
            updateItemProgress(item.id, 20);

            // Step 2: Upload image to the component
            await uploadImageToComponent(item);

            item.status = 'complete';
            completedFiles++;
            updateItemStatus(item.id, 'Complete');
            updateItemProgress(item.id, 100);
            markItemComplete(item.id);

        } catch (error) {
            console.error('Upload failed:', error);
            item.status = 'error';
            completedFiles++;
            updateItemStatus(item.id, 'Failed: ' + error.message);
            markItemError(item.id);
        }

        activeUploads--;
        updateQueueCount();

        // Check if all done
        if (completedFiles >= totalFiles) {
            setTimeout(() => {
                finishUploads();
            }, 1000);
        } else {
            // Process next item
            processQueue();
        }
    }

    /**
     * Create an empty image component via AJAX
     * Posts to the current edit page URL with action=add_component
     */
    async function createImageComponent(file) {
        // Generate a slug from the filename
        const slug = file.name
            .replace(/\.[^/.]+$/, '') // Remove extension
            .toLowerCase()
            .replace(/[^a-z0-9-_]/g, '-') // Replace invalid chars
            .replace(/-+/g, '-') // Remove multiple dashes
            .replace(/^-|-$/g, '') // Remove leading/trailing dashes
            || 'image-' + Date.now(); // Fallback if slug is empty

        // Determine format from file type
        const formatMap = {
            'image/jpeg': 'jpg',
            'image/png': 'png',
            'image/gif': 'gif',
            'image/webp': 'webp',
            'image/svg+xml': 'svg'
        };
        const format = formatMap[file.type] || 'jpg';

        // Build URL-encoded form data (server expects application/x-www-form-urlencoded)
        const params = new URLSearchParams();
        params.append('component_type', 'image');
        // Image components require slug, format, and file_path fields
        params.append('content', JSON.stringify({
            slug: slug,
            format: format,
            file_path: 'pending-upload', // Placeholder until actual upload
            title: file.name,
            description: '',
            alt_text: ''
        }));
        params.append('action', 'add_component');
        params.append('ajax', 'true'); // Signal we want JSON response

        // Get CSRF token if present
        const csrfInput = document.querySelector('input[name="csrf_token"]');
        if (csrfInput) {
            params.append('csrf_token', csrfInput.value);
        }

        // Post to the current URL (the edit page, e.g., /page/.edit)
        const response = await fetch(window.location.href, {
            method: 'POST',
            headers: {
                'Accept': 'application/json',
                'Content-Type': 'application/x-www-form-urlencoded'
            },
            credentials: 'same-origin', // Ensure cookies are sent
            body: params.toString()
        });

        if (!response.ok) {
            // Try to get more details from the response
            let errorDetail = '';
            try {
                const text = await response.text();
                errorDetail = text.substring(0, 200); // First 200 chars
                console.error('Server error response:', text);
            } catch (e) {
                // Ignore parse error
            }
            throw new Error(`Failed to create component (${response.status}): ${errorDetail}`);
        }

        const data = await response.json();

        if (!data.success || !data.component_id) {
            throw new Error(data.error || 'Failed to create component');
        }

        return data.component_id;
    }

    /**
     * Upload image to a component
     */
    function uploadImageToComponent(item) {
        return new Promise((resolve, reject) => {
            const file = item.file;

            // Generate slug from filename
            const slug = file.name
                .replace(/\.[^/.]+$/, '') // Remove extension
                .toLowerCase()
                .replace(/[^a-z0-9-_]/g, '-') // Replace invalid chars
                .replace(/-+/g, '-') // Remove multiple dashes
                .replace(/^-|-$/g, ''); // Remove leading/trailing dashes

            const formData = new FormData();
            formData.append('image', file);
            formData.append('slug', slug);
            formData.append('title', file.name);
            formData.append('description', '');
            formData.append('component_id', item.componentId);

            const xhr = new XMLHttpRequest();

            // Track upload progress
            xhr.upload.addEventListener('progress', (e) => {
                if (e.lengthComputable) {
                    // Scale progress: 20% (component created) to 95% (upload done)
                    const percent = 20 + Math.round((e.loaded / e.total) * 75);
                    updateItemProgress(item.id, percent);
                }
            });

            xhr.onload = function() {
                if (xhr.status === 200) {
                    try {
                        const result = JSON.parse(xhr.responseText);
                        if (result.success) {
                            resolve(result);
                        } else {
                            reject(new Error(result.error || 'Upload failed'));
                        }
                    } catch (parseError) {
                        reject(new Error('Invalid response'));
                    }
                } else {
                    const errorMsg = xhr.status === 413 ? 'File too large' :
                                    xhr.status === 415 ? 'Invalid format' :
                                    `Server error (${xhr.status})`;
                    reject(new Error(errorMsg));
                }
            };

            xhr.onerror = function() {
                reject(new Error('Network error'));
            };

            xhr.open('POST', './.upload-component-image');
            xhr.send(formData);
        });
    }

    /**
     * Update item status in UI
     */
    function updateItemStatus(itemId, status) {
        const itemEl = document.getElementById(itemId);
        if (itemEl) {
            const statusEl = itemEl.querySelector('.item-status');
            if (statusEl) {
                statusEl.textContent = status;
            }
        }
    }

    /**
     * Update item progress in UI
     */
    function updateItemProgress(itemId, percent) {
        const itemEl = document.getElementById(itemId);
        if (itemEl) {
            const progressBar = itemEl.querySelector('.progress-bar');
            if (progressBar) {
                progressBar.style.width = percent + '%';
            }
        }
    }

    /**
     * Mark item as complete in UI
     */
    function markItemComplete(itemId) {
        const itemEl = document.getElementById(itemId);
        if (itemEl) {
            itemEl.classList.add('complete');
        }
    }

    /**
     * Mark item as error in UI
     */
    function markItemError(itemId) {
        const itemEl = document.getElementById(itemId);
        if (itemEl) {
            itemEl.classList.add('error');
        }
    }

    /**
     * Finish uploads and reload page
     */
    function finishUploads() {
        const successCount = uploadQueue.filter(i => i.status === 'complete').length;
        const errorCount = uploadQueue.filter(i => i.status === 'error').length;

        // Update header
        const titleEl = queuePanel.querySelector('.queue-title');
        if (titleEl) {
            if (errorCount === 0) {
                titleEl.textContent = 'Upload complete!';
            } else {
                titleEl.textContent = `${successCount} uploaded, ${errorCount} failed`;
            }
        }

        // Reload page after short delay to show new components
        if (successCount > 0) {
            setTimeout(() => {
                window.location.reload();
            }, 1500);
        } else {
            // No successful uploads, just hide panel after delay
            setTimeout(() => {
                hideQueuePanel();
            }, 3000);
        }
    }

    /**
     * Show a notification
     */
    function showNotification(type, message) {
        // Use SweetAlert2 if available
        if (typeof Swal !== 'undefined') {
            Swal.fire({
                icon: type === 'warning' ? 'warning' : 'info',
                title: type === 'warning' ? 'Warning' : 'Info',
                text: message,
                confirmButtonText: 'OK'
            });
        } else {
            alert(message);
        }
    }

    /**
     * Escape HTML to prevent XSS
     */
    function escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    // Initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
