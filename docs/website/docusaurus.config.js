module.exports = {
  title: 'PowerTo Documentation',
  tagline: 'Documentation for the PowerTo collaborative platform',
  url: 'https://your-docusaurus-site.example.com',
  baseUrl: '/',
  favicon: 'img/logo.svg',
  organizationName: 'ortisan',
  projectName: 'power-to',
  themeConfig: {
    navbar: {
      title: 'PowerTo Docs',
      logo: {
        alt: 'PowerTo Logo',
        src: 'img/logo.svg',
      },
      items: [
        {
          to: '/',
          activeBasePath: 'docs',
          label: 'Documentation',
          position: 'left',
        },
        {
          to: '/architecture/overview',
          label: 'Architecture',
          position: 'left',
        },
        {
          to: '/development/formatter-setup',
          label: 'Development',
          position: 'left',
        },
        {
          to: '/models/issue-proposal-model',
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
              to: '/',
            },
            {
              label: 'Architecture',
              to: '/architecture/overview',
            },
            {
              label: 'Development',
              to: '/development/formatter-setup',
            },
            {
              label: 'Models',
              to: '/models/issue-proposal-model',
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
          routeBasePath: '/',
          exclude: ['website/**'],
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
