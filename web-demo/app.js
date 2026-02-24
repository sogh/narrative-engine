// Narrative Engine — Web Demo
// Loads the WASM module and drives the interactive UI.

import init, { NarrativeDemo } from "./pkg/narrative_engine_wasm.js";

// --- DOM refs ---
const genreSelect = document.getElementById("genre-select");
const seedInput = document.getElementById("seed-input");
const resetBtn = document.getElementById("reset-btn");
const subjectSelect = document.getElementById("subject-select");
const objectSelect = document.getElementById("object-select");
const moodSelect = document.getElementById("mood-select");
const stakesSelect = document.getElementById("stakes-select");
const fnSelect = document.getElementById("fn-select");
const generateBtn = document.getElementById("generate-btn");
const variantsBtn = document.getElementById("variants-btn");
const determinismBtn = document.getElementById("determinism-btn");
const entityCards = document.getElementById("entity-cards");
const outputSection = document.getElementById("output-section");
const outputArea = document.getElementById("output-area");
const loadingEl = document.getElementById("loading");

let demo = null;

// --- Populate a <select> from a JSON array string ---
function populateSelect(el, items, placeholder) {
    el.innerHTML = "";
    if (placeholder) {
        const opt = document.createElement("option");
        opt.value = "";
        opt.textContent = placeholder;
        el.appendChild(opt);
    }
    for (const item of items) {
        const opt = document.createElement("option");
        opt.value = item;
        opt.textContent = item.replace(/_/g, " ");
        el.appendChild(opt);
    }
}

// --- Render entity cards + update subject/object dropdowns ---
function renderScenario() {
    const scenario = JSON.parse(demo.get_scenario());
    entityCards.innerHTML = "";

    // Sort entities by id for stable ordering
    const entities = scenario.entities.sort((a, b) => a.id - b.id);

    for (const e of entities) {
        const card = document.createElement("div");
        card.className = "entity-card";
        card.innerHTML = `
            <div class="name">${e.name}</div>
            <div class="pronouns">${e.pronouns}</div>
            <div class="tags">${e.tags.map(t => `<span class="tag">${t}</span>`).join("")}</div>
        `;
        entityCards.appendChild(card);
    }

    // Populate subject/object selects
    subjectSelect.innerHTML = "";
    objectSelect.innerHTML = '<option value="">(none)</option>';
    for (const e of entities) {
        const opt1 = document.createElement("option");
        opt1.value = e.id;
        opt1.textContent = e.name;
        subjectSelect.appendChild(opt1);

        const opt2 = document.createElement("option");
        opt2.value = e.id;
        opt2.textContent = e.name;
        objectSelect.appendChild(opt2);
    }
}

// --- Build event JSON from form state ---
function buildEventJson() {
    const event = {
        subject_id: parseInt(subjectSelect.value, 10),
        mood: moodSelect.value,
        stakes: stakesSelect.value,
        narrative_fn: fnSelect.value,
    };
    if (objectSelect.value) {
        event.object_id = parseInt(objectSelect.value, 10);
    }
    return JSON.stringify(event);
}

// --- Show output ---
function showOutput(html) {
    outputSection.classList.remove("hidden");
    outputArea.innerHTML = html;
    outputSection.scrollIntoView({ behavior: "smooth", block: "nearest" });
}

// --- Update narrative function dropdown for the current genre ---
function updateFunctionSelect() {
    const supported = JSON.parse(demo.supported_functions());
    const prev = fnSelect.value;
    populateSelect(fnSelect, supported);
    // Keep previous selection if still valid, otherwise pick the first
    if (supported.includes(prev)) {
        fnSelect.value = prev;
    } else {
        fnSelect.value = supported[0] || "";
    }
}

// --- Create an engine instance ---
function createEngine() {
    const genre = genreSelect.value;
    const seed = parseInt(seedInput.value, 10) || 0;
    demo = new NarrativeDemo(genre, BigInt(seed));
    renderScenario();
    updateFunctionSelect();
    outputSection.classList.add("hidden");
}

// --- Event handlers ---
function onGenreChange() {
    createEngine();
}

function onReset() {
    createEngine();
}

function onGenerate() {
    try {
        const text = demo.narrate(buildEventJson());
        showOutput(`<div class="output-block">${escapeHtml(text)}</div>`);
    } catch (e) {
        showOutput(`<div class="output-block" style="color:red">${escapeHtml(e.message || String(e))}</div>`);
    }
}

function onVariants() {
    try {
        const json = demo.narrate_variants(buildEventJson(), 3);
        const variants = JSON.parse(json);
        let html = "";
        variants.forEach((v, i) => {
            html += `<div class="output-label">Variant ${i + 1}</div>`;
            html += `<div class="output-block">${escapeHtml(v)}</div>`;
        });
        showOutput(html);
    } catch (e) {
        showOutput(`<div class="output-block" style="color:red">${escapeHtml(e.message || String(e))}</div>`);
    }
}

function onDeterminism() {
    try {
        const seed = parseInt(seedInput.value, 10) || 0;
        const eventJson = buildEventJson();
        const genre = genreSelect.value;

        // Generate with a fresh engine
        const demo1 = new NarrativeDemo(genre, BigInt(seed));
        const result1 = demo1.narrate(eventJson);

        // Generate again with the same seed
        const demo2 = new NarrativeDemo(genre, BigInt(seed));
        const result2 = demo2.narrate(eventJson);

        const match = result1 === result2;
        let html = "";
        html += `<div class="output-label">Run 1 (seed: ${seed})</div>`;
        html += `<div class="output-block">${escapeHtml(result1)}</div>`;
        html += `<div class="output-label">Run 2 (seed: ${seed})</div>`;
        html += `<div class="output-block">${escapeHtml(result2)}</div>`;
        html += `<div class="determinism-match ${match ? "pass" : "fail"}">`;
        html += match
            ? "Deterministic: outputs are identical"
            : "NOT deterministic: outputs differ (this is a bug!)";
        html += `</div>`;
        showOutput(html);

        // Free the temporary engines
        demo1.free();
        demo2.free();
    } catch (e) {
        showOutput(`<div class="output-block" style="color:red">${escapeHtml(e.message || String(e))}</div>`);
    }
}

function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
}

// --- Init ---
async function main() {
    loadingEl.classList.remove("hidden");

    try {
        await init();

        // Populate static dropdowns
        populateSelect(genreSelect, JSON.parse(NarrativeDemo.available_genres()));
        populateSelect(moodSelect, JSON.parse(NarrativeDemo.moods()));
        populateSelect(stakesSelect, JSON.parse(NarrativeDemo.stakes()));

        // Set sensible defaults
        moodSelect.value = "tense";
        stakesSelect.value = "high";

        // Create initial engine (also populates genre-specific function dropdown)
        createEngine();

        // Wire up events
        genreSelect.addEventListener("change", onGenreChange);
        resetBtn.addEventListener("click", onReset);
        generateBtn.addEventListener("click", onGenerate);
        variantsBtn.addEventListener("click", onVariants);
        determinismBtn.addEventListener("click", onDeterminism);
    } catch (e) {
        loadingEl.textContent = "Failed to load WASM module: " + (e.message || e);
        loadingEl.classList.remove("hidden");
        console.error(e);
        return;
    }

    loadingEl.classList.add("hidden");
}

main();
