// Simple javascript script to generate the download urls from the language selection.

const TYPES = [
    ["main", "Main"],
    ["glossary", "Glossary"],
    ["ipa", "IPA"],
    ["ipa-merged", "IPA merged"],
];

const PAIR_TYPES = ["main", "glossary", "ipa"];

function availableTargets(metadata, source) {
    const targets = new Set();
    for (const type of PAIR_TYPES) {
        for (const target of Object.keys(metadata[type]?.sources?.[source]?.targets ?? {})) {
            targets.add(target);
        }
    }
    return targets;
}

function availableSources(metadata, target) {
    const sources = new Set();
    for (const type of PAIR_TYPES) {
        const bySource = metadata[type]?.sources ?? {};
        for (const source of Object.keys(bySource)) {
            if (target in (bySource[source].targets ?? {})) sources.add(source);
        }
    }
    return sources;
}

function filterDropdown(box, allowed) {
    box.querySelectorAll("div[data-value]").forEach(div => {
        div.dataset.matchesFilter = (!allowed || allowed.has(div.dataset.value)) ? "1" : "0";
        div.style.display = (div.dataset.matchesFilter === "1" && div.dataset.matchesSearch !== "0") ? "" : "none";
    });
}

// Cf. src/path.rs::dict_name_expanded
function buildUrl(type, source, target) {
    const BASE_URL =
        "https://huggingface.co/datasets/daxida/wty-release/resolve/main/latest/dict";
    switch (type) {
        case "main":
            return `${BASE_URL}/${source}/${target}/wty-${source}-${target}.zip`;

        case "ipa":
            return `${BASE_URL}/${source}/${target}/wty-${source}-${target}-ipa.zip`;

        case "ipa-merged":
            return `${BASE_URL}/all/${target}/wty-${target}-ipa.zip`;

        case "glossary":
            return `${BASE_URL}/${source}/${target}/wty-${source}-${target}-gloss.zip`;
    }
}

// Every dictionary that exists for the pair, in display order.
function dictsForPair(metadata, source, target) {
    const found = [];
    for (const [type, label] of TYPES) {
        const merged = type === "ipa-merged";
        const size = merged
            ? metadata[type]?.sources?.all?.targets?.[source]
            : metadata[type]?.sources?.[source]?.targets?.[target];
        if (!size) continue;
        found.push({ label, size, url: buildUrl(type, source, merged ? source : target) });
    }
    return found;
}

// Converts a combobox wrapper into an interactive searchable dropdown.
// Replaces <option> tags with clickable <div>s and wires up filtering on input.
function setupCombobox(box) {
    if (!box) return;
    const search = box.querySelector("input:not([type=hidden])");
    const dropdown = box.querySelector(".dl-source-dropdown, .dl-target-dropdown");
    const hidden = box.querySelector("input[type=hidden]");
    const items = Array.from(dropdown.querySelectorAll("option"));

    // Render items as divs for clicking
    dropdown.innerHTML = "";
    items.forEach((opt) => {
        const div = document.createElement("div");
        div.textContent = opt.textContent;
        div.dataset.value = opt.value;
        div.addEventListener("mousedown", () => {
            search.value = opt.textContent;
            hidden.value = opt.value;
            hidden.dataset.label = opt.textContent;
            dropdown.style.display = "none";
            hidden.dispatchEvent(new Event("change", { bubbles: true }));
        });
        dropdown.appendChild(div);
    });

    function clearSelection() {
        hidden.value = "";
        hidden.dispatchEvent(new Event("change", { bubbles: true }));
    }

    search.addEventListener("focus", () => (dropdown.style.display = "block"));
    search.addEventListener("blur", () =>
        setTimeout(() => (dropdown.style.display = "none"), 150),
    );
    search.addEventListener("input", () => {
        const q = search.value.toLowerCase();
        dropdown.style.display = "block";

        // We set matchesSearch and read matchesFilter from filterDropdown
        dropdown.querySelectorAll("div").forEach((div) => {
            div.dataset.matchesSearch = div.textContent.toLowerCase().includes(q) ? "1" : "0";
            div.style.display = (div.dataset.matchesFilter !== "0" && div.dataset.matchesSearch === "1") ? "" : "none";
        });

        // Clear selection if user edits away from selected label
        if (hidden.value && search.value !== hidden.dataset.label) {
            clearSelection();
        }

        if (!q) clearSelection();
    });

    // Complete with first match when clicking the "Enter" key
    search.addEventListener("keydown", (e) => {
        if (e.key !== "Enter") return;
        e.preventDefault();

        const firstVisible = Array.from(
            dropdown.querySelectorAll("div")
        ).find(div => div.style.display !== "none");
        if (!firstVisible) return;

        search.value = firstVisible.textContent;
        hidden.value = firstVisible.dataset.value;
        dropdown.style.display = "none";
        hidden.dispatchEvent(new Event("change", { bubbles: true }));
    });
}

function renderResult({ label, size, url }) {
    const name = document.createElement("span");
    name.className = "dl-name";
    name.textContent = label;

    const sizeEl = document.createElement("span");
    sizeEl.className = "dl-size";
    sizeEl.textContent = size;

    const labelEl = document.createElement("div");
    labelEl.className = "dl-label";
    labelEl.append(name, sizeEl);

    const link = document.createElement("a");
    link.className = "dl-btn";
    link.href = `${url}?download=true`;
    link.textContent = "↓";
    link.title = "Download";

    const code = document.createElement("code");
    code.textContent = url;

    const copyBtn = document.createElement("button");
    copyBtn.textContent = "Copy";
    copyBtn.title = "Copy URL";
    copyBtn.className = "copy-url-btn";
    copyBtn.onclick = async () => {
        try {
            await navigator.clipboard.writeText(url);
            copyBtn.textContent = "Copied";
            setTimeout(() => (copyBtn.textContent = "Copy"), 1500);
        } catch {
            copyBtn.textContent = "Failed";
        }
    };

    const block = document.createElement("div");
    block.className = "dl-url";
    block.append(copyBtn, code);

    const li = document.createElement("li");
    li.append(labelEl, link, block);
    return li;
}

// Wires up the page: initialises both comboboxes and lists the dictionaries of the chosen pair.
function setupPage(table, metadata) {
    const sourceHidden = table.querySelector(".dl-source");
    const targetHidden = table.querySelector(".dl-target");
    const results = document.querySelector(".dl-results");

    table.querySelectorAll(".dl-source-combobox, .dl-target-combobox").forEach(
        setupCombobox,
    );

    function showMessage(text) {
        const message = document.createElement("div");
        message.className = "dl-empty";
        message.textContent = text;
        results.replaceChildren(message);
    }

    function update() {
        const source = sourceHidden.value;
        const target = targetHidden.value;

        filterDropdown(
            table.querySelector(".dl-target-combobox"),
            source ? availableTargets(metadata, source) : null,
        );
        filterDropdown(
            table.querySelector(".dl-source-combobox"),
            target ? availableSources(metadata, target) : null,
        );

        if (!source || !target) {
            showMessage("Select the languages");
            return;
        }

        const found = dictsForPair(metadata, source, target);
        if (found.length === 0) {
            showMessage("No dictionary for this pair");
            return;
        }

        const list = document.createElement("ul");
        list.append(...found.map(renderResult));
        results.replaceChildren(list);
    }

    sourceHidden.addEventListener("change", update);
    targetHidden.addEventListener("change", update);

    update();
}

// Mojo so that fetching works both locally and in a project repo
// There MUST be a better way to do this...
const REPO_NAME = "wiktionary-to-yomitan";
const BRANCH = "gh-pages"; // branch that serves the site
const base = document.querySelector('base')?.href || `https://yomidevs.github.io/${REPO_NAME}/`;
const metadataPromise = fetch(`${base}release_metadata.json`)
    .then(res => res.json())
    .then(json => json["dicts"]);

// I don't think this is ideal (it is called on every tab switch, and not only on the download's one),
// but it's the only thing I got working...
// cf. https://github.com/squidfunk/mkdocs-material/discussions/6788#discussioncomment-8498415
document$.subscribe(function () {
    metadataPromise.then((metadata) => {
        const table = document.querySelector(".download-table");
        if (!table) return;

        // Mark table as loaded to fade it in
        table.classList.add("loaded");

        setupPage(table, metadata);
    });
});
