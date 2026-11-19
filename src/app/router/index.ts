import { createRouter, createWebHashHistory } from 'vue-router'

const ProjectsPage = () => import('../../modules/projects/ProjectsPage.vue')
const ProjectPage = () => import('../../modules/projects/ProjectPage.vue')
const SettingsPage = () => import('../../modules/settings/SettingsPage.vue')
const VerificationPage = () => import('../../modules/verification/VerificationPage.vue')
const ModelsPage = () => import('../../modules/settings/ModelsPage.vue')
const WritingPage = () => import('../../modules/writing/WritingPage.vue')
const CoversPage = () => import('../../modules/covers/CoversPage.vue')
const PublishingPage = () => import('../../modules/publishing/PublishingPage.vue')
const PublishQueuePage = () => import('../../modules/publishing/PublishQueuePage.vue')
const AboutPage = () => import('../../modules/settings/AboutPage.vue')
const VideoPage = () => import('../../modules/video/VideoPage.vue')

export const router = createRouter({
  history: createWebHashHistory(),
  scrollBehavior(to, from, savedPosition) {
    if (savedPosition) return savedPosition
    if (to.path !== from.path) return { top: 0 }
  },
  routes: [
    { path: '/', redirect: '/projects' },
    { path: '/projects', component: ProjectsPage },
    { path: '/projects/new', component: ProjectPage },
    { path: '/publishing', component: PublishQueuePage },
    { path: '/projects/:id', component: ProjectPage, props: true },
    { path: '/projects/:id/writing', component: WritingPage, props: true },
    { path: '/projects/:id/covers', component: CoversPage, props: true },
    { path: '/projects/:id/video', component: VideoPage, props: true },
    { path: '/projects/:id/publishing', component: PublishingPage, props: route => ({ id: route.params.id, taskId: typeof route.query.task === 'string' ? route.query.task : undefined }) },
    { path: '/settings/models', component: ModelsPage },
    { path: '/settings', component: SettingsPage },
    { path: '/about', component: AboutPage },
    { path: '/verification', component: VerificationPage },
    { path: '/:pathMatch(.*)*', redirect: '/projects' },
  ],
})
