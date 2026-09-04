export default {
  lang: 'en-US',
  title: 'RustyCore',
  description: 'RustyCore project documentation',
  base: '/rustycore/',
  cleanUrls: true,
  appearance: 'force-dark',
  srcExclude: ['README.md'],
  head: [['link', { rel: 'icon', href: '/rustycore/logo.svg' }]],
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
