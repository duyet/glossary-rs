// API Base URL
const API_BASE = '/api/v1';

// State
let allGlossary = {};
let searchTimeout = null;
let currentEditId = null;

// Theme Management
function initTheme() {
    const savedTheme = localStorage.getItem('theme') || 'light';
    document.documentElement.setAttribute('data-theme', savedTheme);
}

function toggleTheme() {
    const current = document.documentElement.getAttribute('data-theme');
    const next = current === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    localStorage.setItem('theme', next);
}

// API Functions
async function fetchGlossary() {
    try {
        const response = await fetch(`${API_BASE}/glossary`);
        if (!response.ok) throw new Error('Failed to fetch glossary');
        return await response.json();
    } catch (error) {
        console.error('Error fetching glossary:', error);
        showError('Failed to load glossary');
        return {};
    }
}

async function searchGlossary(query) {
    try {
        const response = await fetch(`${API_BASE}/glossary-search?q=${encodeURIComponent(query)}`);
        if (!response.ok) throw new Error('Search failed');
        return await response.json();
    } catch (error) {
        console.error('Error searching:', error);
        return { results: [], count: 0 };
    }
}

async function fetchPopular() {
    try {
        const response = await fetch(`${API_BASE}/glossary-popular?limit=10`);
        if (!response.ok) throw new Error('Failed to fetch popular terms');
        return await response.json();
    } catch (error) {
        console.error('Error fetching popular terms:', error);
        return [];
    }
}

async function createTerm(term, definition) {
    try {
        const response = await fetch(`${API_BASE}/glossary`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'x-authenticated-user-email': getUserEmail()
            },
            body: JSON.stringify({ term, definition })
        });
        if (!response.ok) throw new Error('Failed to create term');
        return await response.json();
    } catch (error) {
        console.error('Error creating term:', error);
        throw error;
    }
}

async function updateTerm(id, term, definition) {
    try {
        const response = await fetch(`${API_BASE}/glossary/${id}`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
                'x-authenticated-user-email': getUserEmail()
            },
            body: JSON.stringify({ term, definition })
        });
        if (!response.ok) throw new Error('Failed to update term');
        return await response.json();
    } catch (error) {
        console.error('Error updating term:', error);
        throw error;
    }
}

async function likeTerm(id) {
    try {
        const response = await fetch(`${API_BASE}/glossary/${id}/likes`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'x-authenticated-user-email': getUserEmail()
            },
            body: JSON.stringify({})
        });
        if (!response.ok) throw new Error('Failed to like term');
        return await response.json();
    } catch (error) {
        console.error('Error liking term:', error);
        throw error;
    }
}

function getUserEmail() {
    // In a real app, this would come from authentication
    return localStorage.getItem('userEmail') || 'user@example.com';
}

// Render Functions
function renderAlphabetGrid(glossary) {
    const grid = document.getElementById('alphabetGrid');
    grid.innerHTML = '';

    const sorted = Object.keys(glossary).sort();

    if (sorted.length === 0) {
        showEmptyState();
        return;
    }

    sorted.forEach(letter => {
        const section = document.createElement('div');
        section.className = 'alphabet-section';

        const heading = document.createElement('h2');
        heading.textContent = letter;
        section.appendChild(heading);

        const list = document.createElement('div');
        list.className = 'terms-list';

        glossary[letter].forEach(term => {
            const card = createTermCard(term);
            list.appendChild(card);
        });

        section.appendChild(list);
        grid.appendChild(section);
    });

    grid.style.display = 'grid';
}

function createTermCard(term) {
    const card = document.createElement('div');
    card.className = 'term-card';

    const title = document.createElement('h3');
    title.textContent = term.term;
    card.appendChild(title);

    const definition = document.createElement('p');
    definition.textContent = term.definition;
    card.appendChild(definition);

    const meta = document.createElement('div');
    meta.className = 'term-meta';
    meta.innerHTML = `
        <span>❤️ ${term.likes_count || 0} likes</span>
        ${term.who ? `<span>👤 ${term.who}</span>` : ''}
    `;
    card.appendChild(meta);

    // Click to like
    card.addEventListener('click', async () => {
        try {
            await likeTerm(term.id);
            loadGlossary(); // Refresh
        } catch (error) {
            console.error('Error liking term:', error);
        }
    });

    return card;
}

function renderSearchResults(results) {
    const container = document.getElementById('searchResults');
    container.innerHTML = '';

    if (results.results.length === 0) {
        container.innerHTML = '<p style="text-align: center; color: var(--color-text-secondary); padding: 2rem;">No results found</p>';
    } else {
        const list = document.createElement('div');
        list.className = 'terms-list';

        results.results.forEach(term => {
            const card = createTermCard(term);
            list.appendChild(card);
        });

        container.appendChild(list);
    }

    container.style.display = 'block';
    document.getElementById('alphabetGrid').style.display = 'none';
    document.getElementById('searchStats').textContent = `Found ${results.count} term${results.count !== 1 ? 's' : ''}`;
}

async function renderPopular() {
    const container = document.getElementById('popularList');
    container.innerHTML = '<p style="color: var(--color-text-secondary); font-size: 0.875rem;">Loading...</p>';

    const popular = await fetchPopular();

    container.innerHTML = '';

    popular.forEach(term => {
        const item = document.createElement('div');
        item.className = 'popular-item';

        const title = document.createElement('h4');
        title.textContent = term.term;
        item.appendChild(title);

        const meta = document.createElement('p');
        meta.textContent = `❤️ ${term.likes_count} likes`;
        item.appendChild(meta);

        item.addEventListener('click', () => {
            // Scroll to term in main view
            const searchInput = document.getElementById('searchInput');
            searchInput.value = term.term;
            handleSearch();
        });

        container.appendChild(item);
    });

    if (popular.length === 0) {
        container.innerHTML = '<p style="color: var(--color-text-secondary); font-size: 0.875rem;">No popular terms yet</p>';
    }
}

// Modal Functions
function openModal(editMode = false, term = null) {
    const modal = document.getElementById('termModal');
    const title = document.getElementById('modalTitle');
    const termInput = document.getElementById('termName');
    const defInput = document.getElementById('termDefinition');

    if (editMode && term) {
        title.textContent = 'Edit Term';
        termInput.value = term.term;
        defInput.value = term.definition;
        currentEditId = term.id;
    } else {
        title.textContent = 'New Term';
        termInput.value = '';
        defInput.value = '';
        currentEditId = null;
    }

    modal.classList.add('active');
}

function closeModal() {
    document.getElementById('termModal').classList.remove('active');
    currentEditId = null;
}

async function handleFormSubmit(e) {
    e.preventDefault();

    const term = document.getElementById('termName').value.trim();
    const definition = document.getElementById('termDefinition').value.trim();

    try {
        if (currentEditId) {
            await updateTerm(currentEditId, term, definition);
        } else {
            await createTerm(term, definition);
        }

        closeModal();
        await loadGlossary();
    } catch (error) {
        showError('Failed to save term');
    }
}

// Search Handler
async function handleSearch() {
    const query = document.getElementById('searchInput').value.trim();
    const clearBtn = document.getElementById('searchClear');

    clearBtn.style.display = query ? 'block' : 'none';

    if (!query) {
        document.getElementById('searchResults').style.display = 'none';
        document.getElementById('alphabetGrid').style.display = 'grid';
        document.getElementById('searchStats').textContent = '';
        return;
    }

    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(async () => {
        const results = await searchGlossary(query);
        renderSearchResults(results);
    }, 300);
}

// Utility Functions
function showLoading() {
    document.getElementById('loading').style.display = 'flex';
    document.getElementById('alphabetGrid').style.display = 'none';
    document.getElementById('emptyState').style.display = 'none';
}

function hideLoading() {
    document.getElementById('loading').style.display = 'none';
}

function showEmptyState() {
    document.getElementById('emptyState').style.display = 'block';
    document.getElementById('alphabetGrid').style.display = 'none';
}

function showError(message) {
    // Simple error display - could be enhanced with a toast notification
    alert(message);
}

// Load Initial Data
async function loadGlossary() {
    showLoading();
    allGlossary = await fetchGlossary();
    renderAlphabetGrid(allGlossary);
    renderPopular();
    hideLoading();
}

// Event Listeners
document.addEventListener('DOMContentLoaded', () => {
    // Initialize theme
    initTheme();

    // Load initial data
    loadGlossary();

    // Theme toggle
    document.getElementById('themeToggle').addEventListener('click', toggleTheme);

    // New term button
    document.getElementById('newTermBtn').addEventListener('click', () => openModal(false));

    // Modal controls
    document.getElementById('modalClose').addEventListener('click', closeModal);
    document.getElementById('cancelBtn').addEventListener('click', closeModal);

    // Modal backdrop click
    document.getElementById('termModal').addEventListener('click', (e) => {
        if (e.target.id === 'termModal') closeModal();
    });

    // Form submit
    document.getElementById('termForm').addEventListener('submit', handleFormSubmit);

    // Search
    const searchInput = document.getElementById('searchInput');
    searchInput.addEventListener('input', handleSearch);

    // Clear search
    document.getElementById('searchClear').addEventListener('click', () => {
        searchInput.value = '';
        handleSearch();
    });

    // Empty state button
    document.querySelector('.empty-state button')?.addEventListener('click', () => openModal(false));
});
