<template>
  <div>
    <div class="sm:flex sm:items-center">
      <div class="sm:flex-auto">
        <h1 class="text-base font-semibold leading-6 text-gray-900">Accounts</h1>
      </div>
      <div class="mt-4 sm:ml-16 sm:mt-0 sm:flex-none">
        <button
          type="button"
          class="block rounded-md bg-indigo-600 px-3 py-2 text-center text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
        >
          Add account
        </button>
      </div>
    </div>
    <div class="mt-8 flow-root">
      <div class="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
        <div class="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
          <table class="table-auto divide-y divide-gray-300">
            <thead>
              <tr>
                <th
                  scope="col"
                  class="py-3 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 sm:pl-0"
                >
                  <a href="#" class="group inline-flex">
                    Number
                    <span
                      class="invisible ml-2 flex-none rounded text-gray-400 group-hover:visible group-focus:visible"
                    >
                      <ChevronDownIcon class="h-5 w-5" aria-hidden="true" />
                    </span>
                  </a>
                </th>
                <th
                  scope="col"
                  class="px-3 py-3 text-left text-sm font-semibold text-gray-900"
                >
                  <a href="#" class="group inline-flex">
                    Name
                    <span
                      class="ml-2 flex-none rounded bg-gray-100 text-gray-900 group-hover:bg-gray-200"
                    >
                      <ChevronDownIcon class="h-5 w-5" aria-hidden="true" />
                    </span>
                  </a>
                </th>
                <th
                  scope="col"
                  class="px-3 py-3 text-center text-sm font-semibold text-gray-900"
                >
                  <a href="#" class="group inline-flex">
                    Balances
                    <!-- <span
                      class="invisible ml-2 flex-none rounded text-gray-400 group-hover:visible group-focus:visible">
                      <ChevronDownIcon
                        class="invisible ml-2 h-5 w-5 flex-none rounded text-gray-400 group-hover:visible group-focus:visible"
                        aria-hidden="true" />
                    </span> -->
                  </a>
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200 bg-white">
              <tr
                v-for="account in accounts"
                :key="account.id"
                class="odd:bg-gray-50 even:bg-white"
              >
                <td
                  class="align-top whitespace-nowrap py-2 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-0"
                >
                  <RouterLink
                    :to="{
                      name: 'account-register',
                      params: { ledger_id: account.ledger_id, account_id: account.id }
                    }"
                  >
                    {{ account.number }}
                  </RouterLink>
                </td>
                <td
                  class="align-top whitespace-nowrap px-3 py-2 text-sm font-medium text-gray-900"
                >
                  <RouterLink
                    :to="{
                      name: 'account-register',
                      params: { ledger_id: account.ledger_id, account_id: account.id }
                    }"
                  >
                    {{ account.name }}
                  </RouterLink>
                </td>
                <td class="align-top whitespace-nowrap px-3 py-2 text-sm text-gray-900">
                  <table v-if="account.balances" class="w-full">
                    <tbody>
                      <tr
                        v-for="currency in Object.keys(account.balances)"
                        :key="currency"
                      >
                        <td class="text-right w-1/2 pr-1">
                          {{ account.balances[currency] }}
                        </td>
                        <td class="w-1/2">{{ currency }}</td>
                      </tr>
                    </tbody>
                  </table>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ChevronDownIcon } from '@heroicons/vue/20/solid'

const accounts = [
  {
    id: 2723,
    ledger_id: 1594,
    parent_id: null,
    name: 'Asset',
    normal: 'DR',
    number: 1000,
    version: 346555748,
    balances: {
      MSFT: '5',
      USD: '116086'
    }
  },
  {
    id: 4270,
    ledger_id: 1594,
    parent_id: null,
    name: 'Liability',
    normal: 'CR',
    number: 2000,
    version: null,
    balances: {}
  },
  {
    id: 6786,
    ledger_id: 1594,
    parent_id: null,
    name: 'Equity',
    normal: 'CR',
    number: 3000,
    version: 27335,
    balances: {
      CAD: '48',
      MSFT: '5',
      USD: '-1923'
    }
  },
  {
    id: 5737,
    ledger_id: 1594,
    parent_id: null,
    name: 'Income',
    normal: 'CR',
    number: 4000,
    version: 346558700,
    balances: {
      USD: '118009'
    }
  },
  {
    id: 3720,
    ledger_id: 1594,
    parent_id: null,
    name: 'Expense',
    normal: 'DR',
    number: 5000,
    version: 19439,
    balances: {
      CAD: '48'
    }
  }
]
</script>
