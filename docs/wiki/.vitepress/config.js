export default {
  lang: 'en-US',
  title: "RustyCore",
  description: "RustyCore Documentation",
  head: [['link', { rel: 'icon', href: '/logo.svg' }]],
  appearance: 'force-dark',
  base: '/rustycore/', // Needs to be changed later if Rustycore become an github org
  cleanUrls: true,
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Client', link: '/client/', activeMatch: '/client/' },
      { text: 'Server', link: '/server/', activeMatch: '/server/' },
      { text: 'Develop', link: '/develop/', activeMatch: '/develop/' },
      { text: 'Reference', link: '/reference/', activeMatch: '/reference/' },

    ],
    sidebar: [
      {
        text: 'Client',
        collapsed: false,
        base: '/client/',
        items: [
          { text: 'Overview', link: 'index' },
          { text: 'Setup', link: 'setup' },
        ]
      },
      {
        text: 'Server',
        collapsed: false,
        base: '/server/',
        items: [
          { text: 'Overview', link: 'index' },
          { text: 'Setup', link: 'setup' },
        ]
      },
      {
        text: 'Develop',
        collapsed: false,
        base: '/develop/',
        items: [
          { text: 'Overview', link: 'index' },
          {
            text: 'Sub Cat 2',
            items: [
              { text: 'Test', link: 'test' },
              { text: 'Test', link: 'test' }
            ]
          },
          {
            text: 'Sub Cat 2',
            items: [
              { text: 'Test', link: 'test' },
              { text: 'Test', link: 'test' }
            ]
          },
        ]
      },
            {
        text: 'Reference',
        collapsed: false,
        base: '/reference/',
        items: [
          { text: 'Overview', link: 'index' },
          { text: 'Config', link: 'config' },
        ]
      }
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/alseif0x/rustycore' },
      { icon: 'discord', link: 'https://discord.gg/mH6ACpGPb2' }
    ],
    editLink: {
      pattern: 'https://github.com/alseif0x/rustycore/tree/3.4.3/docs/wiki/:path',
      text: 'Edit this page on GitHub'
    },
  }
}
