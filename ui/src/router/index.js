import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      components: { default: HomeView }
    },
    {
      path: '/ledger/:ledger_id',
      name: 'ledger',
      // route level code-splitting
      // this generates a separate chunk (Ledger.[hash].js) for this route
      // which is lazy-loaded when the route is visited.
      components: {
        default: () => import('../views/LedgerView.vue')
      }
    },
    {
      path: '/ledger/:ledger_id/accounts/:account_id/register',
      name: 'account-register',
      // route level code-splitting
      // this generates a separate chunk (Ledger.[hash].js) for this route
      // which is lazy-loaded when the route is visited.
      components: {
        default: () => import('../views/AccountView.vue')
      }
    }
  ]
})

export default router
