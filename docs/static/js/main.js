window.addEventListener("DOMContentLoaded", () => {
	const CONSTANTS = {
		SCROLL_THRESHOLD: 300,
		COPY_FEEDBACK_DELAY: 2000,
		SEARCH_SCORE_THRESHOLD: 0.6,
		BLUR_DELAY: 50
	};

	if (!window.searchIndex) {
		console.error('Search index not found');
		return;
	}

	const fuse = new Fuse(window.searchIndex, {
		keys: ["title", "description"],
		includeScore: true,
		includeMatches: true,
	});

	// Hide/show the scroll-to-top button
	const scrollButton = document.getElementById("scroll-to-top");
	const fillElement = document.getElementById("scroll-fill");
	const scrollElement = document.getElementById("scroll-element");

	if (!scrollButton || !fillElement || !scrollElement) {
		console.warn('Scroll elements not found');
		return;
	}

	let enabled = false;
	let scrollTimeout;

	scrollElement.addEventListener("scroll", (e) => {
		if (scrollTimeout) {
			clearTimeout(scrollTimeout);
		}
		scrollTimeout = setTimeout(() => {
			const percent =
				(e.target.scrollTop / (e.target.scrollHeight - e.target.clientHeight)) * 100;
			fillElement.style.width = `${percent}%`;
			if (e.target.scrollTop > CONSTANTS.SCROLL_THRESHOLD) {
				scrollButton.style.opacity = 1;
				scrollButton.style.cursor = "pointer";
				enabled = true;
			} else {
				scrollButton.style.opacity = 0;
				scrollButton.style.cursor = "default";
				enabled = false;
			}
		}, 16);
	});

	scrollButton.addEventListener("click", () => {
		if (!enabled) return;
		scrollElement.scrollTo({
			top: 0,
			behavior: "smooth",
		});
	});

	// Add copy buttons to code blocks (data-copy)
	const codeBlockStart = performance.now();
	const preElements = document.querySelectorAll("pre[data-copy]");

	for (const preElement of preElements) {
		const codeElement = preElement.querySelector("code");
		if (!codeElement) continue;

		const copyButton = document.createElement("button");
		copyButton.className = "copy-button";
		copyButton.textContent = "Copy";
		copyButton.setAttribute('aria-label', 'Copy code to clipboard');

		copyButton.addEventListener("click", async () => {
			try {
				if (navigator.clipboard) {
					await navigator.clipboard.writeText(codeElement.textContent);
				} else {
					const textArea = document.createElement('textarea');
					textArea.value = codeElement.textContent;
					document.body.appendChild(textArea);
					textArea.select();
					document.execCommand('copy');
					document.body.removeChild(textArea);
				}
				copyButton.textContent = "Copied!";
				setTimeout(() => {
					copyButton.textContent = "Copy";
				}, CONSTANTS.COPY_FEEDBACK_DELAY);
			} catch (err) {
				console.error('Failed to copy text: ', err);
				copyButton.textContent = "Error";
				setTimeout(() => {
					copyButton.textContent = "Copy";
				}, CONSTANTS.COPY_FEEDBACK_DELAY);
			}
		});
		preElement.appendChild(copyButton);
	}
	console.log(
		`Copy buttons injection took ${(performance.now() - codeBlockStart).toFixed(2)} ms`
	);

	const twemojiStart = performance.now();
	twemoji.parse(document.body);
	console.log(`twemoji.parse() took ${(performance.now() - twemojiStart).toFixed(2)} ms`);

	const input = document.getElementById("goto-input");
	const results = document.getElementById("goto-results");
	const popup = document.getElementById("goto-popup");

	if (!input || !results || !popup) {
		console.warn('Search elements not found');
		return;
	}

	// Add accessibility attributes
	input.setAttribute('role', 'combobox');
	input.setAttribute('aria-expanded', 'false');
	input.setAttribute('aria-autocomplete', 'list');
	input.setAttribute('aria-haspopup', 'listbox');
	results.setAttribute('role', 'listbox');
	results.setAttribute('aria-label', 'Search results');

	let selectedIndex = -1;
	let searchResults = [];
	let searchTimeout;

	function highlightResult(index) {
		const resultElements = results.querySelectorAll(".goto-result");

		for (const el of resultElements) {
			el.classList.remove("selected");
		}

		if (index >= 0 && index < resultElements.length) {
			resultElements[index].classList.add("selected");
			resultElements[index].scrollIntoView({ block: "nearest", behavior: "smooth" });
		}
	}

	function performSearch() {
		const query = input.value.trim();
		searchResults = query ? fuse.search(query) : [];
		searchResults = searchResults
			.filter((result) => result.score < CONSTANTS.SEARCH_SCORE_THRESHOLD)
			.sort((a, b) => a.score - b.score)
			.filter((v, i, a) => a.findIndex((t) => t.item.title === v.item.title) === i);
		results.innerHTML = "";

		if (searchResults.length === 0 && query) {
			const noResult = document.createElement("div");
			noResult.className = "goto-no-result";
			noResult.textContent = "No results found";
			results.appendChild(noResult);
			return;
		}

		for (const { item } of searchResults) {
			const result = document.createElement("a");
			result.href = item.url;
			result.className = "goto-result";
			result.innerHTML = `<h3>${item.title}</h3>`;
			result.setAttribute('role', 'option');
			result.setAttribute('tabindex', '-1');

			// Add mouse interactions
			result.addEventListener("mouseenter", () => {
				selectedIndex = Array.from(results.children).indexOf(result);
				highlightResult(selectedIndex);
			});

			results.appendChild(result);
		}

		// Reset selection
		selectedIndex = searchResults.length > 0 ? 0 : -1;
		highlightResult(selectedIndex);
	}

	function navigateToResult(index) {
		if (index >= 0 && index < searchResults.length) {
			window.location.href = searchResults[index].item.url;
		}
	}

	let buttonClicked = false;

	input.addEventListener("input", () => {
		if (searchTimeout) {
			clearTimeout(searchTimeout);
		}
		searchTimeout = setTimeout(() => {
			performSearch();
		}, 150);
	});

	input.addEventListener("keydown", (e) => {
		switch (e.key) {
			case "ArrowDown":
				e.preventDefault();
				if (searchResults.length > 0) {
					selectedIndex = (selectedIndex + 1) % searchResults.length;
					highlightResult(selectedIndex);
				}
				break;

			case "ArrowUp":
				e.preventDefault();
				if (searchResults.length > 0) {
					selectedIndex =
						(selectedIndex - 1 + searchResults.length) % searchResults.length;
					highlightResult(selectedIndex);
				}
				break;

			case "Enter":
				e.preventDefault();
				navigateToResult(selectedIndex);
				break;

			case "Escape":
				e.preventDefault();
				popup.classList.add("hidden");
				input.setAttribute('aria-expanded', 'false');
				input.blur();
				break;
		}
	});

	input.addEventListener("blur", (e) => {
		if (!buttonClicked && !popup.contains(e.relatedTarget)) {
			console.log("blur,hidePopup");
			setTimeout(() => {
				popup.classList.add("hidden");
				input.setAttribute('aria-expanded', 'false');
			}, CONSTANTS.BLUR_DELAY);
		}
		buttonClicked = false;
	});

	document.addEventListener("keydown", (e) => {
		if (e.key === "k" && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			if (popup.classList.contains("hidden")) {
				popup.classList.remove("hidden");
				input.setAttribute('aria-expanded', 'true');
				input.focus();
				input.select();
			} else {
				popup.classList.add("hidden");
				input.setAttribute('aria-expanded', 'false');
				input.blur();
			}
		}

		if (
			e.key === "/" &&
			document.activeElement.tagName !== "INPUT" &&
			document.activeElement.tagName !== "TEXTAREA"
		) {
			e.preventDefault();
			popup.classList.remove("hidden");
			input.setAttribute('aria-expanded', 'true');
			input.focus();
			input.select();
		}

		if (e.key === "Escape" && !popup.classList.contains("hidden")) {
			e.preventDefault();
			popup.classList.add("hidden");
			input.setAttribute('aria-expanded', 'false');
			input.blur();
		}
	});

	const gotoButton = document.getElementById("goto-button");
	if (gotoButton) {
		gotoButton.addEventListener("click", () => {
			buttonClicked = true;
			popup.classList.toggle("hidden");
			if (!popup.classList.contains("hidden")) {
				input.focus();
				input.select();
			}
		});
	}

	document.addEventListener("touchstart", (e) => {
		if (
			!popup.classList.contains("hidden") &&
			!popup.contains(e.target) &&
			e.target !== gotoButton
		) {
			console.log("touchstart,hidePopup");
			popup.classList.add("hidden");
			input.setAttribute('aria-expanded', 'false');
		}
	});
});
