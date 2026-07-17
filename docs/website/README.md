# PowerTo Documentation Site

This directory contains the [Docusaurus](https://docusaurus.io/) configuration for the PowerTo documentation site, as well as the documentation content itself.

## Structure

- `docusaurus.config.js`: Main configuration file for Docusaurus
- `sidebars.js`: Sidebar configuration for the documentation
- `src/`: Custom React components and CSS
- `static/`: Static assets like images and favicon
- `../architecture/`, `../product/`, `../development/`, `../models/`: Documentation content organized by category
- `intro.md`: Introduction page for the documentation

## Getting Started

To run the documentation site locally:

```bash
# Install dependencies
cd docs/website
npm ci

# Start the development server
npm start
```

This will start a local development server and open up a browser window. Most changes are reflected live without having to restart the server.

## Building

To build the documentation site for production:

```bash
# Build the site
cd docs/website
npm run build
```

This will generate static content in the `build` directory that can be served using any static content hosting service.

## Serving

To serve the built website locally:

```bash
# Serve the built site
cd docs/website
npm run serve
```

This will start a local server to serve the built site.

## Customizing

### Adding New Documentation

1. Add Markdown files to the appropriate subdirectory under the parent `docs` directory
2. Update `sidebars.js` to include your new documentation
3. Restart the development server to see your changes

### Changing the Theme

1. Edit `src/css/custom.css` to change the theme colors and other styles
2. Update `docusaurus.config.js` to change the site configuration

## Learn More

To learn more about Docusaurus, check out the [official documentation](https://docusaurus.io/).
