import { createRouter, createWebHashHistory } from 'vue-router'
import ProjectsPage from '../../modules/projects/ProjectsPage.vue'
import ProjectPage from '../../modules/projects/ProjectPage.vue'
import SettingsPage from '../../modules/settings/SettingsPage.vue'
import VerificationPage from '../../modules/verification/VerificationPage.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/projects' },
    { path: '/projects', component: ProjectsPage },
    { path: '/projects/new', component: ProjectPage },
    { path: '/projects/:id', component: ProjectPage, props: true },
    { path: '/settings', component: SettingsPage },
    { path: '/verification', component: VerificationPage },
    { path: '/:pathMatch(.*)*', redirect: '/projects' },
  ],
})
