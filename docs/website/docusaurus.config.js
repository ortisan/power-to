module.exports = {
  title: 'PowerTo Documentation',
  tagline: 'Documentation for the PowerTo collaborative platform',
  url: 'https://your-docusaurus-site.example.com',
  baseUrl: '/',
  favicon: 'img/logo.svg',
  organizationName: 'power-to', // Usually your GitHub org/user name.
  projectName: 'power-to', // Usually your repo name.
  themeConfig: {
    navbar: {
      title: 'PowerTo Docs',
      logo: {
        alt: 'PowerTo Logo',
        src: 'img/logo.svg',
      },
      items: [
        {
          to: 'intro',
          activeBasePath: 'docs',
          label: 'Documentation',
          position: 'left',
        },
        {
          to: 'architecture/decisions/0000-adr-template',
          label: 'Architecture',
          position: 'left',
        },
        {
          to: 'development/formatter-setup',
          label: 'Development',
          position: 'left',
        },
        {
          to: 'models/issue-proposal-model',
          label: 'Models',
          position: 'left',
        },
        {
          href: 'https://github.com/ortisan/power-to',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Introduction',
              to: 'intro',
            },
            {
              label: 'Architecture',
              to: 'architecture/decisions/0000-adr-template',
            },
            {
              label: 'Development',
              to: 'development/formatter-setup',
            },
            {
              label: 'Models',
              to: 'models/issue-proposal-model',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/ortisan/power-to',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} PowerTo Contributors. Built with Docusaurus.`,
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          path: '../',
          sidebarPath: require.resolve('./sidebars.js'),
          editUrl: 'https://github.com/ortisan/power-to/edit/main/docs/',
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
};
