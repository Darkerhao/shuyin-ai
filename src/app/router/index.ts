import { createRouter, createWebHashHistory } from 'vue-router'
import VerificationPage from '../../modules/verification/VerificationPage.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/verification' },
    { path: '/verification', component: VerificationPage },
    { path: '/:pathMatch(.*)*', redirect: '/verification' },
  ],
})
