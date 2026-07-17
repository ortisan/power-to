module.exports = {
  docs: [
    {
      type: 'doc',
      id: 'intro',
      label: 'Introduction',
    },
    {
      type: 'category',
      label: 'Product',
      items: [
        'product/mvp-scope',
      ],
    },
    {
      type: 'category',
      label: 'Architecture',
      items: [
        'architecture/overview',
        'architecture/technology-stack',
        'architecture/media-storage',
        'architecture/mobile-sensing',
        {
          type: 'category',
          label: 'Decisions',
          items: [
            'architecture/decisions/0000-adr-template',
            'architecture/decisions/0001-semantic-versioning-and-conventional-commits',
            'architecture/decisions/0002-blockchain-for-voting',
            'architecture/decisions/0003-opensource-observability-tools',
            'architecture/decisions/0004-rust-backend',
            'architecture/decisions/0005-diesel-persistence',
            'architecture/decisions/0006-modular-monolith',
            'architecture/decisions/0007-postgresql-postgis',
            'architecture/decisions/0008-relational-vote-ledger',
            'architecture/decisions/0009-clean-architecture',
            'architecture/decisions/0010-opentelemetry-victoria-observability',
            'architecture/decisions/0011-portable-media-storage',
            'architecture/decisions/0012-atlas-database-migrations',
            'architecture/decisions/0013-mobile-capture-and-road-sensing',
            'architecture/decisions/0014-kotlin-multiplatform-native-mobile',
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
