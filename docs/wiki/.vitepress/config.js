export default {
  lang: 'en-US',
  title: 'RustyCore',
  description: 'Bringing Azeroth to Rust. An open-source WotLK Classic 3.4.3 server emulator — setup guides, documentation and ways to contribute.',
  base: '/rustycore/',
  cleanUrls: true,
  appearance: true,
  srcExclude: ['README.md'],
  head: [
    ['link', { rel: 'icon', href: '/rustycore/logo.svg', type: 'image/svg+xml' }],
    ['meta', { name: 'theme-color', content: '#0a111b' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:site_name', content: 'RustyCore' }],
    ['meta', { property: 'og:image', content: 'https://alseif0x.github.io/rustycore/rustycore-banner.png' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    search: { provider: 'local' },
    outline: { level: [2, 3], label: 'On this page' },
    footer: {
      message: 'Open source under <a href="https://github.com/alseif0x/rustycore/blob/3.4.3/LICENSE">GPL-3.0-or-later</a>. Built by the RustyCore community.',
      copyright: 'World of Warcraft belongs to Blizzard Entertainment. RustyCore is an independent project.',
    },
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Client', link: '/client/', activeMatch: '/client/' },
      { text: 'Server', link: '/server/', activeMatch: '/server/' },
      { text: 'Contribute', link: '/develop/', activeMatch: '/develop/' },
      { text: 'Reference', link: '/reference/', activeMatch: '/reference/' },
    ],
    sidebar: [
      {
        text: 'Client',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/client/' },
          { text: 'Setup', link: '/client/setup' },
        ],
      },
      {
        text: 'Server',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/server/' },
          { text: 'Setup', link: '/server/setup' },
        ],
      },
      {
        text: 'Develop',
        collapsed: false,
        items: [{ text: 'Contributing', link: '/develop/' }],
      },
      {
        text: 'Reference',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/reference/' },
          { text: 'Configuration', link: '/reference/config' },
        ],
      },
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/alseif0x/rustycore' },
      { icon: 'discord', link: 'https://discord.gg/mH6ACpGPb2' },
    ],
    editLink: {
      pattern: 'https://github.com/alseif0x/rustycore/edit/3.4.3/docs/wiki/:path',
      text: 'Edit this page on GitHub',
    },
  },
}
