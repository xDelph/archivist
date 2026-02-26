export interface ThreadMessage {
  id: string
  author: {
    name: string
    initials: string
    color: string
  }
  message: string
  timestamp: string
  reactions?: number
  hasFile?: boolean
  fileType?: "image" | "pdf" | "code" | "link"
  fileName?: string
}

export interface SlackThread {
  id: string
  author: {
    name: string
    initials: string
    color: string
  }
  channel: string
  channelColor: string
  message: string
  date: string
  replies: number
  reactions: number
  participants: number
  score: number
  hasFiles: boolean
  fileType?: "image" | "pdf" | "code" | "link"
  url?: string
  threadMessages: ThreadMessage[]
}

export interface ChannelStat {
  name: string
  color: string
  count: number
  percentage: number
}

export interface ActivityPoint {
  date: string
  messages: number
  threads: number
}

const channelColors: Record<string, string> = {
  "#mods": "bg-chart-1/15 text-chart-1",
  "#announcements": "bg-chart-2/15 text-chart-2",
  "#random": "bg-chart-3/15 text-chart-3",
  "#help": "bg-chart-5/15 text-chart-5",
  "#introductions": "bg-chart-2/15 text-chart-2",
  "#freelance": "bg-chart-3/15 text-chart-3",
  "#general": "bg-chart-1/15 text-chart-1",
  "#design": "bg-info/15 text-info",
  "#engineering": "bg-chart-4/15 text-chart-4",
  "#showcase": "bg-warning/15 text-warning",
}

export const allTimeThreads: SlackThread[] = [
  {
    id: "1",
    author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" },
    channel: "#mods",
    channelColor: channelColors["#mods"],
    message: "Bonjour, voici le site https://www.chatoulain.com/ , j'aimerais avoir vos retours dessus !",
    date: "Jan 24, 2026",
    replies: 47,
    reactions: 89,
    participants: 23,
    score: 983,
    hasFiles: false,
    url: "https://www.chatoulain.com",
    threadMessages: [
      { id: "1-1", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "Super clean le design ! Par contre le hero me semble un peu chargé, vous avez pensé a simplifier ?", timestamp: "10:32 AM", reactions: 12 },
      { id: "1-2", author: { name: "Bernard", initials: "BE", color: "bg-chart-2" }, message: "Je suis d'accord avec Line. Le call-to-action est pas assez visible. Mettez un bouton plus gros et en contraste.", timestamp: "10:45 AM", reactions: 8 },
      { id: "1-3", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "Merci pour les retours ! On a refait le hero en suivant vos conseils. Le CTA est maintenant en vert fluo sur fond sombre.", timestamp: "11:12 AM", reactions: 15 },
      { id: "1-4", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "Le temps de chargement est un peu lent sur mobile. Avez-vous compressé les images ? Je recommande WebP.", timestamp: "11:30 AM", reactions: 6 },
      { id: "1-5", author: { name: "Greg Lindelher", initials: "GL", color: "bg-info" }, message: "J'ai testé sur Safari et y a un bug de layout sur la section pricing. Le grid se casse en dessous de 768px.", timestamp: "12:05 PM", reactions: 4, hasFile: true, fileType: "image", fileName: "safari-bug.png" },
    ],
  },
  {
    id: "2",
    author: { name: "Bernard", initials: "BE", color: "bg-chart-2" },
    channel: "#announcements",
    channelColor: channelColors["#announcements"],
    message: "C'est un des choses qui brisent le plafond d'un double code, et on plus il a des outils qui ont les pages qui deviennent grises...",
    date: "Dec 23, 2025",
    replies: 32,
    reactions: 65,
    participants: 18,
    score: 876,
    hasFiles: false,
    threadMessages: [
      { id: "2-1", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "C'est un vrai problème récurrent. On devrait faire un guide communautaire pour éviter ces erreurs.", timestamp: "2:15 PM", reactions: 9 },
      { id: "2-2", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "J'avais le même souci la semaine dernière. Le fix c'est de vider le cache et de recompiler.", timestamp: "2:28 PM", reactions: 14 },
      { id: "2-3", author: { name: "Bernard", initials: "BE", color: "bg-chart-2" }, message: "Merci Line, ça a marché ! Je documente ça dans le wiki pour les autres.", timestamp: "3:05 PM", reactions: 7 },
    ],
  },
  {
    id: "3",
    author: { name: "Patrick Syrykoulnik", initials: "PS", color: "bg-chart-3" },
    channel: "#mods",
    channelColor: channelColors["#mods"],
    message: "Decouvrez ce que l'on fait sur https://www.monobook.com/ et dites-nous ce que vous en pensez !",
    date: "Jan 24, 2026",
    replies: 55,
    reactions: 91,
    participants: 34,
    score: 1240,
    hasFiles: false,
    url: "https://www.monobook.com",
    threadMessages: [
      { id: "3-1", author: { name: "Alexandre Berner", initials: "AB", color: "bg-chart-4" }, message: "L'UX est vraiment bien pensée, bravo ! Le onboarding est fluide et intuitif.", timestamp: "9:00 AM", reactions: 18 },
      { id: "3-2", author: { name: "Maxime Laverty", initials: "ML", color: "bg-chart-3" }, message: "Le pricing est un peu élevé pour les indépendants. Vous avez prévu un plan starter ?", timestamp: "9:15 AM", reactions: 11 },
      { id: "3-3", author: { name: "Patrick Syrykoulnik", initials: "PS", color: "bg-chart-3" }, message: "Oui on a un plan gratuit qui arrive ce mois-ci ! Merci pour le feedback.", timestamp: "9:30 AM", reactions: 22 },
      { id: "3-4", author: { name: "Olivier Lotte", initials: "OL", color: "bg-chart-4" }, message: "La feature de collaboration en temps réel est impressionnante. Comment vous gérez le sync ?", timestamp: "10:02 AM", reactions: 7 },
      { id: "3-5", author: { name: "Patrick Syrykoulnik", initials: "PS", color: "bg-chart-3" }, message: "On utilise des CRDTs avec Yjs pour le real-time. C'est ultra performant même avec 50+ users simultanés.", timestamp: "10:20 AM", reactions: 25 },
      { id: "3-6", author: { name: "Damien", initials: "DA", color: "bg-chart-1" }, message: "Est-ce que vous avez une API publique ? J'aimerais intégrer ça dans notre workflow.", timestamp: "11:05 AM", reactions: 5 },
    ],
  },
  {
    id: "4",
    author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" },
    channel: "#help",
    channelColor: channelColors["#help"],
    message: "Comment je peux configurer mon reverse proxy ? J'ai eu un ticket ouvert mais je n'arrive pas a contourner le probleme de CORS...",
    date: "Feb 5, 2026",
    replies: 124,
    reactions: 45,
    participants: 28,
    score: 1100,
    hasFiles: true,
    fileType: "code",
    threadMessages: [
      { id: "4-1", author: { name: "Greg Lindelher", initials: "GL", color: "bg-info" }, message: "Tu utilises quel reverse proxy ? Nginx ou Caddy ? Avec Caddy le CORS se configure en 3 lignes.", timestamp: "3:00 PM", reactions: 8 },
      { id: "4-2", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "C'est Nginx. Voici ma config actuelle:", timestamp: "3:12 PM", reactions: 3, hasFile: true, fileType: "code", fileName: "nginx.conf" },
      { id: "4-3", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "Le problème c'est que tu n'as pas ajouté les headers Access-Control-Allow-Origin. Ajoute ça dans le location block.", timestamp: "3:25 PM", reactions: 15 },
      { id: "4-4", author: { name: "Maxime Laverty", initials: "ML", color: "bg-chart-3" }, message: "Aussi vérifie que tu passes les preflight OPTIONS requests. C'est souvent ça qui bloque.", timestamp: "3:40 PM", reactions: 12 },
      { id: "4-5", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "Ca marche ! C'était bien le preflight. Merci beaucoup, je mets la config finale ici pour les autres.", timestamp: "4:15 PM", reactions: 20, hasFile: true, fileType: "code", fileName: "nginx-fixed.conf" },
      { id: "4-6", author: { name: "Alexandre Berner", initials: "AB", color: "bg-chart-4" }, message: "Je bookmark ce thread. C'est le 4ème message CORS qu'on a ce mois. On devrait pin ça.", timestamp: "4:30 PM", reactions: 9 },
      { id: "4-7", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "Epinglé dans #help ! Merci tout le monde.", timestamp: "4:45 PM", reactions: 6 },
    ],
  },
  {
    id: "5",
    author: { name: "Bernard", initials: "BE", color: "bg-chart-2" },
    channel: "#random",
    channelColor: channelColors["#random"],
    message: "Est-ce que quelqu'un a deja essaye ce nouveau framework? Il fait des benchmarks assez fous...",
    date: "Jan 18, 2026",
    replies: 38,
    reactions: 72,
    participants: 19,
    score: 790,
    hasFiles: true,
    fileType: "link",
    threadMessages: [
      { id: "5-1", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "Les benchmarks sont ouf mais c'est toujours pareil, en prod c'est autre chose...", timestamp: "11:00 AM", reactions: 8 },
      { id: "5-2", author: { name: "Bernard", initials: "BE", color: "bg-chart-2" }, message: "En fait j'ai testé en prod et c'est vraiment plus rapide. Le cold start est 3x meilleur que l'alternative.", timestamp: "11:20 AM", reactions: 14 },
      { id: "5-3", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "Intéressant. Par contre la documentation est quasi inexistante. Quelqu'un a trouvé des bons tutos ?", timestamp: "11:45 AM", reactions: 6 },
    ],
  },
  {
    id: "6",
    author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" },
    channel: "#announcements",
    channelColor: channelColors["#announcements"],
    message: "Attention, petite update: demain au soir il faut terminer le dernier push avant la release de la version 4.2...",
    date: "Jan 22, 2026",
    replies: 12,
    reactions: 31,
    participants: 9,
    score: 520,
    hasFiles: false,
    threadMessages: [
      { id: "6-1", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "Bien reçu ! On est prêts de notre côté. Le staging est validé.", timestamp: "5:00 PM", reactions: 4 },
      { id: "6-2", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "Pareil ici, les tests passent tous. On merge demain matin ?", timestamp: "5:15 PM", reactions: 3 },
    ],
  },
  {
    id: "7",
    author: { name: "Alexandre Berner", initials: "AB", color: "bg-chart-4" },
    channel: "#freelance",
    channelColor: channelColors["#freelance"],
    message: "J'ai enfin trouve le moyen de gagner plus de 10k par mois en freelance, voici mes conseils...",
    date: "Jan 21, 2026",
    replies: 67,
    reactions: 120,
    participants: 42,
    score: 1560,
    hasFiles: true,
    fileType: "pdf",
    threadMessages: [
      { id: "7-1", author: { name: "Damien", initials: "DA", color: "bg-chart-1" }, message: "Tu peux partager plus de détails sur ta stratégie d'acquisition clients ?", timestamp: "8:00 AM", reactions: 15 },
      { id: "7-2", author: { name: "Alexandre Berner", initials: "AB", color: "bg-chart-4" }, message: "Bien sûr ! Voici mon guide complet en PDF. Le plus important c'est le positionnement de niche.", timestamp: "8:30 AM", reactions: 28, hasFile: true, fileType: "pdf", fileName: "freelance-guide-2026.pdf" },
      { id: "7-3", author: { name: "Francois Dussert", initials: "FD", color: "bg-chart-2" }, message: "Merci pour le partage ! Question: tu utilises quoi pour la facturation et la compta ?", timestamp: "9:00 AM", reactions: 7 },
      { id: "7-4", author: { name: "Alexandre Berner", initials: "AB", color: "bg-chart-4" }, message: "Pennylane pour la compta et Stripe pour la facturation. Le combo est parfait.", timestamp: "9:20 AM", reactions: 19 },
      { id: "7-5", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "10k/mois en freelance c'est faisable mais ça demande de la discipline. +1 sur le positionnement de niche.", timestamp: "10:00 AM", reactions: 11 },
    ],
  },
  {
    id: "8",
    author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" },
    channel: "#mods",
    channelColor: channelColors["#mods"],
    message: "La stack technique optimale pour un SaaS en 2026: NextJS, Supabase, Stripe, Vercel. Voici pourquoi...",
    date: "Feb 10, 2026",
    replies: 89,
    reactions: 156,
    participants: 51,
    score: 1890,
    hasFiles: true,
    fileType: "image",
    threadMessages: [
      { id: "8-1", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "100% d'accord pour NextJS + Supabase. Mais pourquoi pas Drizzle au lieu de l'ORM Supabase ?", timestamp: "7:30 AM", reactions: 12 },
      { id: "8-2", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "Drizzle est top aussi mais Supabase client offre le real-time et le RLS out of the box. Pour un SaaS c'est critique.", timestamp: "7:45 AM", reactions: 24 },
      { id: "8-3", author: { name: "Maxime Laverty", initials: "ML", color: "bg-chart-3" }, message: "J'ajouterais Resend pour les emails transactionnels. Le combo avec React Email est parfait.", timestamp: "8:10 AM", reactions: 18 },
      { id: "8-4", author: { name: "Greg Lindelher", initials: "GL", color: "bg-info" }, message: "Et pour le monitoring ? Sentry + Vercel Analytics ca suffit ?", timestamp: "8:30 AM", reactions: 6 },
      { id: "8-5", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "Oui c'est ce que j'utilise. Voici l'architecture complete en image:", timestamp: "9:00 AM", reactions: 31, hasFile: true, fileType: "image", fileName: "saas-architecture-2026.png" },
      { id: "8-6", author: { name: "Alexandre Berner", initials: "AB", color: "bg-chart-4" }, message: "Ce thread est une mine d'or. Je sauvegarde tout. Merci Quentin !", timestamp: "9:30 AM", reactions: 14 },
      { id: "8-7", author: { name: "Bernard", initials: "BE", color: "bg-chart-2" }, message: "Quelqu'un a un template de depart avec cette stack ? Ca ferait gagner un temps fou.", timestamp: "10:00 AM", reactions: 9 },
      { id: "8-8", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "J'y travaille justement ! Je prepare un starter kit open source. Stay tuned.", timestamp: "10:30 AM", reactions: 42 },
    ],
  },
  {
    id: "9",
    author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" },
    channel: "#introductions",
    channelColor: channelColors["#introductions"],
    message: "Un peu genant de se presenter mais je suis en train de monter mon propre SaaS...",
    date: "Nov 20, 2025",
    replies: 22,
    reactions: 53,
    participants: 15,
    score: 620,
    hasFiles: false,
    threadMessages: [
      { id: "9-1", author: { name: "Alexandre Berner", initials: "AB", color: "bg-chart-4" }, message: "Bienvenue ! C'est quoi ton SaaS ? On est tous curieux ici.", timestamp: "2:00 PM", reactions: 8 },
      { id: "9-2", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "C'est un outil de gestion de projets pour les freelances. Je suis en phase de prototypage.", timestamp: "2:20 PM", reactions: 12 },
      { id: "9-3", author: { name: "Damien", initials: "DA", color: "bg-chart-1" }, message: "Super idee ! N'hesite pas a partager tes avancees dans #showcase quand tu seras prete.", timestamp: "2:45 PM", reactions: 6 },
    ],
  },
  {
    id: "10",
    author: { name: "Greg Lindelher", initials: "GL", color: "bg-info" },
    channel: "#help",
    channelColor: channelColors["#help"],
    message: "Pour les personnes qui sont en contrats pret chez les mairies, vous devriez eviter cette app qui...",
    date: "Feb 14, 2026",
    replies: 15,
    reactions: 23,
    participants: 8,
    score: 410,
    hasFiles: false,
    threadMessages: [
      { id: "10-1", author: { name: "Francois Dussert", initials: "FD", color: "bg-chart-2" }, message: "Merci pour le warning. J'allais justement l'installer. Quelle alternative tu recommandes ?", timestamp: "4:00 PM", reactions: 5 },
      { id: "10-2", author: { name: "Greg Lindelher", initials: "GL", color: "bg-info" }, message: "Utilisez plutot l'outil officiel de la prefecture. C'est gratuit et plus fiable.", timestamp: "4:15 PM", reactions: 9 },
    ],
  },
  {
    id: "11",
    author: { name: "Maxime Laverty", initials: "ML", color: "bg-chart-3" },
    channel: "#mods",
    channelColor: channelColors["#mods"],
    message: "Mon meilleur outil pour coder: https://cursor.sh - Changez-moi la vie de dev en solo avec un agent IA...",
    date: "Feb 1, 2026",
    replies: 78,
    reactions: 134,
    participants: 45,
    score: 1670,
    hasFiles: false,
    url: "https://cursor.sh",
    threadMessages: [
      { id: "11-1", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "Cursor est vraiment next level. La completion avec Claude est folle de precision.", timestamp: "1:00 PM", reactions: 22 },
      { id: "11-2", author: { name: "Bernard", initials: "BE", color: "bg-chart-2" }, message: "J'utilise aussi et c'est devenu indispensable. Le tab-complete me fait gagner 2h par jour easy.", timestamp: "1:15 PM", reactions: 15 },
      { id: "11-3", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "Par contre attention a pas trop dependre de l'IA. Faut quand meme comprendre ce qu'on code.", timestamp: "1:30 PM", reactions: 28 },
      { id: "11-4", author: { name: "Maxime Laverty", initials: "ML", color: "bg-chart-3" }, message: "Totalement d'accord Florian. C'est un outil, pas un remplacement. Je review toujours le code genere.", timestamp: "1:45 PM", reactions: 19 },
      { id: "11-5", author: { name: "Olivier Lotte", initials: "OL", color: "bg-chart-4" }, message: "Le mode Agent est impressionnant pour le refactoring. Il comprend le contexte du projet entier.", timestamp: "2:00 PM", reactions: 16 },
    ],
  },
  {
    id: "12",
    author: { name: "Olivier Lotte", initials: "OL", color: "bg-chart-4" },
    channel: "#mods",
    channelColor: channelColors["#mods"],
    message: "Une alternative gratuite a un outil payant? Voici ce qui m'a servi: https://github.com/example/oss-tool",
    date: "Jan 28, 2026",
    replies: 41,
    reactions: 68,
    participants: 22,
    score: 820,
    hasFiles: false,
    url: "https://github.com/example/oss-tool",
    threadMessages: [
      { id: "12-1", author: { name: "Damien", initials: "DA", color: "bg-chart-1" }, message: "Merci du partage ! Je cherchais exactement ca. Le self-hosting est simple ?", timestamp: "3:00 PM", reactions: 7 },
      { id: "12-2", author: { name: "Olivier Lotte", initials: "OL", color: "bg-chart-4" }, message: "Oui, un simple docker-compose up et c'est parti. La doc est bien faite.", timestamp: "3:20 PM", reactions: 11 },
      { id: "12-3", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "Je l'ai deploye sur mon VPS en 10 minutes. Fonctionne parfaitement.", timestamp: "3:45 PM", reactions: 9 },
    ],
  },
  {
    id: "13",
    author: { name: "Francois Dussert", initials: "FD", color: "bg-chart-2" },
    channel: "#random",
    channelColor: channelColors["#random"],
    message: "Si quelqu'un veut une premiere beta invite: Vite Hotload c'est le futur! DM moi si interessses...",
    date: "Dec 8, 2025",
    replies: 19,
    reactions: 42,
    participants: 11,
    score: 480,
    hasFiles: false,
    threadMessages: [
      { id: "13-1", author: { name: "Maxime Laverty", initials: "ML", color: "bg-chart-3" }, message: "DM envoye ! Ca a l'air prometteur comme outil.", timestamp: "6:00 PM", reactions: 4 },
      { id: "13-2", author: { name: "Bernard", initials: "BE", color: "bg-chart-2" }, message: "Pareil, je veux bien tester. Ca remplace quoi exactement ?", timestamp: "6:15 PM", reactions: 3 },
    ],
  },
  {
    id: "14",
    author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" },
    channel: "#random",
    channelColor: channelColors["#random"],
    message: "Il parait qu'on met pas react. Depuis 7/25, Claude Code et l'IA remplace tout... je suis dubitatif...",
    date: "Jan 15, 2026",
    replies: 93,
    reactions: 47,
    participants: 33,
    score: 1120,
    hasFiles: false,
    threadMessages: [
      { id: "14-1", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "L'IA va pas remplacer React. Elle va changer comment on l'ecrit. Nuance importante.", timestamp: "11:00 AM", reactions: 25 },
      { id: "14-2", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "On disait pareil de jQuery. En fait l'IA pousse vers plus d'abstraction, pas la fin du code.", timestamp: "11:20 AM", reactions: 18 },
      { id: "14-3", author: { name: "Alexandre Berner", initials: "AB", color: "bg-chart-4" }, message: "Le vrai changement c'est que la barriere d'entree baisse. Plus de gens peuvent coder, c'est positif.", timestamp: "11:45 AM", reactions: 14 },
      { id: "14-4", author: { name: "Maxime Laverty", initials: "ML", color: "bg-chart-3" }, message: "Les devs qui comprennent le fondamental auront toujours de la valeur. L'IA aide, elle ne remplace pas.", timestamp: "12:00 PM", reactions: 31 },
      { id: "14-5", author: { name: "Greg Lindelher", initials: "GL", color: "bg-info" }, message: "Debat interessant mais en attendant, mon Claude Code a crash 3 fois aujourd'hui. On est loin du remplacement total.", timestamp: "12:30 PM", reactions: 22 },
      { id: "14-6", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "Ok vous m'avez convaincu. C'est un outil complementaire, pas un remplacement. Je reste dev React.", timestamp: "1:00 PM", reactions: 15 },
    ],
  },
  {
    id: "15",
    author: { name: "Damien", initials: "DA", color: "bg-chart-1" },
    channel: "#mods",
    channelColor: channelColors["#mods"],
    message: "J'ai besoin d'un retour sur cette landing page: le hero est-il assez impactant pour convertir ?",
    date: "Feb 12, 2026",
    replies: 34,
    reactions: 58,
    participants: 16,
    score: 740,
    hasFiles: true,
    fileType: "image",
    threadMessages: [
      { id: "15-1", author: { name: "Florian Brosseau", initials: "FB", color: "bg-chart-1" }, message: "Le hero est clean mais le CTA est trop bas. Remonte-le au-dessus de la fold.", timestamp: "7:00 PM", reactions: 10 },
      { id: "15-2", author: { name: "Quentin Rulois", initials: "QR", color: "bg-chart-3" }, message: "Le copywriting est un peu generique. 'Revolutionnez votre workflow' ca dit rien. Sois plus specifique.", timestamp: "7:20 PM", reactions: 14 },
      { id: "15-3", author: { name: "Damien", initials: "DA", color: "bg-chart-1" }, message: "Bons points ! Je refais le hero avec un CTA plus haut et du copy plus concret. Voici la V2:", timestamp: "8:00 PM", reactions: 8, hasFile: true, fileType: "image", fileName: "landing-v2.png" },
      { id: "15-4", author: { name: "Line Germaine", initials: "LG", color: "bg-chart-5" }, message: "La V2 est beaucoup mieux ! Le contraste est bien meilleur et le message est plus clair.", timestamp: "8:30 PM", reactions: 12 },
    ],
  },
]

export const weekThreads: SlackThread[] = allTimeThreads
  .filter((_, i) => [3, 7, 9, 10, 14].includes(i))
  .map((t, i) => ({ ...t, score: t.score - i * 50 }))

export const monthThreads: SlackThread[] = allTimeThreads
  .filter((_, i) => [0, 2, 3, 4, 5, 7, 10, 11, 13, 14].includes(i))
  .map((t, i) => ({ ...t, score: t.score - i * 30 }))

export const channelStats: ChannelStat[] = [
  { name: "#mods", color: "bg-chart-1", count: 3420, percentage: 32 },
  { name: "#announcements", color: "bg-chart-2", count: 1280, percentage: 12 },
  { name: "#random", color: "bg-chart-3", count: 2100, percentage: 20 },
  { name: "#help", color: "bg-chart-5", count: 1560, percentage: 15 },
  { name: "#freelance", color: "bg-chart-3", count: 890, percentage: 8 },
  { name: "#introductions", color: "bg-chart-2", count: 650, percentage: 6 },
  { name: "#showcase", color: "bg-warning", count: 430, percentage: 4 },
  { name: "#design", color: "bg-info", count: 320, percentage: 3 },
]

export const activityData: ActivityPoint[] = [
  { date: "Mon", messages: 342, threads: 28 },
  { date: "Tue", messages: 456, threads: 35 },
  { date: "Wed", messages: 389, threads: 31 },
  { date: "Thu", messages: 521, threads: 42 },
  { date: "Fri", messages: 478, threads: 38 },
  { date: "Sat", messages: 123, threads: 9 },
  { date: "Sun", messages: 89, threads: 5 },
]

export const overviewStats = {
  totalMessages: 10650,
  totalThreads: 1847,
  totalFiles: 423,
  totalUsers: 67,
  messagesChange: +12.4,
  threadsChange: +8.2,
  filesChange: +5.7,
  usersChange: +2.1,
}
