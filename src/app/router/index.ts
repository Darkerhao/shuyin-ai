import { createRouter, createWebHashHistory } from 'vue-router'
import ProjectsPage from '../../modules/projects/ProjectsPage.vue'
import ProjectPage from '../../modules/projects/ProjectPage.vue'
import SettingsPage from '../../modules/settings/SettingsPage.vue'
import VerificationPage from '../../modules/verification/VerificationPage.vue'
import ModelsPage from '../../modules/settings/ModelsPage.vue'
import WritingPage from '../../modules/writing/WritingPage.vue'
import CoversPage from '../../modules/covers/CoversPage.vue'
import PublishingPage from '../../modules/publishing/PublishingPage.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/projects' },
    { path: '/projects', component: ProjectsPage },
    { path: '/projects/new', component: ProjectPage },
    { path: '/projects/:id', component: ProjectPage, props: true },
    { path: '/projects/:id/writing', component: WritingPage, props: true },
    { path: '/projects/:id/covers', component: CoversPage, props: true },
    { path: '/projects/:id/publishing', component: PublishingPage, props: true },
    { path: '/settings/models', component: ModelsPage },
    { path: '/settings', component: SettingsPage },
    { path: '/verification', component: VerificationPage },
    { path: '/:pathMatch(.*)*', redirect: '/projects' },
  ],
})
