module.exports = {
  docs: [
    {
      type: 'doc',
      id: 'intro',
      label: 'Introduction',
    },
    {
      type: 'category',
      label: 'Architecture',
      items: [
        {
          type: 'category',
          label: 'Decisions',
          items: [
            'architecture/decisions/0000-adr-template',
            'architecture/decisions/0001-semantic-versioning-and-conventional-commits',
            'architecture/decisions/0002-blockchain-for-voting',
            'architecture/decisions/0003-opensource-observability-tools',
          ],
        },
      ],
    },
    {
      type: 'category',
      label: 'Development',
      items: [
        'development/formatter-setup',
      ],
    },
    {
      type: 'category',
      label: 'Models',
      items: [
        'models/issue-proposal-model',
      ],
    },
  ],
};
