import type { Metadata, Viewport } from 'next'
import { Geist, Geist_Mono } from 'next/font/google'
import { Analytics } from '@vercel/analytics/next'
import './globals.css'

const _geist = Geist({ subsets: ["latin"] });
const _geistMono = Geist_Mono({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: 'Archivist',
  description: 'Save and browse your Slack workspace messages, files, and top threads. Discover the best conversations by week, month, or all time.',
  icons: {
    icon: '/icon.svg',
    shortcut: '/icon.svg',
    apple: '/icon.svg',
  },
  openGraph: {
    type: 'website',
    title: 'Archivist',
    description: 'Save and browse your Slack workspace messages, files, and top threads.',
    images: ['/icon.svg'],
  },
  twitter: {
    card: 'summary',
    title: 'Archivist',
    description: 'Save and browse your Slack workspace messages, files, and top threads.',
    images: ['/icon.svg'],
  },
}

export const viewport: Viewport = {
  themeColor: '#1a1a2e',
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <body className="font-sans antialiased">
        {children}
        <Analytics />
      </body>
    </html>
  )
}
