// Géométrie absente de jsdom ; les parcours graphiques utilisent Playwright.
Range.prototype.getBoundingClientRect = () => new DOMRect()
Range.prototype.getClientRects = () => Object.assign([], { item: () => null })
