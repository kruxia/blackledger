<template>
  <div>
    <div class="sm:flex sm:items-center">
      <div class="sm:flex-auto">
        <h1 class="text-base font-semibold leading-6 text-gray-900">Account: {{ account.name }} <span
            v-if="account.number">({{ account.number }})</span></h1>
      </div>
      <!-- <div class="mt-4 sm:ml-16 sm:mt-0 sm:flex-none">
        <button type="button"
          class="block rounded-md bg-indigo-600 px-3 py-2 text-center text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600">Export</button>
      </div> -->
    </div>
    <div class="mt-8 flow-root">
      <div class="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
        <div class="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
          <table class="min-w-full divide-y divide-gray-300">
            <thead>
              <tr>
                <th scope="col"
                  class="whitespace-nowrap py-3.5 pl-4 pr-3 text-left text-sm font-semibold text-gray-900">
                  Date
                </th>
                <th scope="col" class="whitespace-nowrap px-2 py-3.5 text-left text-sm font-semibold text-gray-900">
                  Memo/Name
                </th>
                <th scope="col" class="whitespace-nowrap px-2 py-3.5 text-right text-sm font-semibold text-gray-900">
                  Debit
                </th>
                <th scope="col" class="whitespace-nowrap px-2 py-3.5 text-right text-sm font-semibold text-gray-900">
                  Credit
                </th>
                <th scope="col" class="whitespace-nowrap px-2 py-3.5 text-left text-sm font-semibold text-gray-900">
                  Curr
                </th>
                <th scope="col" class="relative whitespace-nowrap py-3.5 pl-3 pr-4 text-right text-sm font-semibold text-gray-900">
                  <button v-on:click="txDetails = !txDetails">
                    {{ txDetails ? '&#x2212;' : '&#x002b;' }}
                    Details
                  </button>
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200 bg-white">
              <tr v-for="transaction in transactions" :key="transaction.id" class="odd:bg-gray-100 even:bg-white">
                <!-- Date -->
                <td class="align-text-top whitespace-nowrap py-2 pl-4 pr-3 text-sm text-gray-900">
                  {{ transaction.effective.split('T')[0] }}
                </td>
                <!-- Memo -->
                <td class="align-text-top whitespace-nowrap py-2 pr-0 text-sm text-gray-900">
                  <p v-for="_, i in transaction.entries.filter(e => e.acct == account.id)" class="px-2">
                    <span v-if="i == 0">{{ transaction.memo }}</span>
                    <span v-else>&nbsp;</span>
                  </p>
                  <div v-if="txDetails" class="border-y border-l ml-2 pl-2 text-gray-500 italic">
                    <p v-for="entry, i in transaction.entries.filter(e => e.acct != account.id)">
                      {{ entry.acct_name }}
                    </p>
                  </div>
                </td>
                <!-- Debit -->
                <td class="align-text-top text-right whitespace-nowrap py-2 px-0 text-sm text-gray-900">
                  <p v-for="entry in transaction.entries.filter(e => e.acct == account.id)" class="pr-2">
                    {{ entry.dr || '&nbsp;' }}
                  </p>
                  <div v-if="txDetails" class="border-y text-gray-500">
                    <p v-for="entry in transaction.entries.filter(e => e.acct != account.id)" class="pr-2">
                      {{ entry.dr || '&nbsp;' }}
                    </p>
                  </div>
                </td>
                <!-- Credit -->
                <td class="align-text-top text-right whitespace-nowrap py-2 px-0 text-sm text-gray-900">
                  <p v-for="entry in transaction.entries.filter(e => e.acct == account.id)" class="pr-2">
                    {{ entry.cr || '&nbsp;' }}
                  </p>
                  <div v-if="txDetails" class="border-y text-gray-500">
                    <p v-for="entry in transaction.entries.filter(e => e.acct != account.id)" class="pr-2">
                      {{ entry.cr || '&nbsp;' }}
                    </p>
                  </div>
                </td>
                <!-- Currency -->
                <td class="align-text-top whitespace-nowrap py-2 px-0 text-sm text-gray-900">
                  <p v-for="entry in transaction.entries.filter(e => e.acct == account.id)" class="pl-2">
                    {{ entry.curr }}
                  </p>
                  <div v-if="txDetails" class="border-y border-r text-gray-500">
                    <p v-for="entry in transaction.entries.filter(e => e.acct != account.id)" class="pl-2">
                      {{ entry.curr }}
                    </p>
                  </div>
                </td>
                <!-- Actions -->
                <td class="align-text-top relative whitespace-nowrap py-2 pl-3 pr-4 text-right text-sm font-medium">
                  <a href="#" class="text-indigo-600 hover:text-indigo-900">
                    void
                  </a>
                  <!-- <a href="#" class="ml-1 border-l pl-1 border-l-indigo-300 text-indigo-600 hover:text-indigo-900">Adj</a> -->
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
import { ref } from 'vue'
const account = {
  id: 2723,
  name: "Asset",
  number: 1000,
  ledger_id: 1594
}
const txDetails = ref(false)
const transactions = [
  {
    "id": 7117,
    "ledger_id": 1594,
    "posted": "2024-08-31T00:39:09.532222Z",
    "effective": "2024-08-31T00:39:09.532222Z",
    "memo": "client1",
    "meta": null,
    "entries": [
      {
        "id": 8630,
        "ledger_id": 1594,
        "tx": 7117,
        "acct": 2723,
        "acct_name": "Asset",
        "dr": "1000",
        "cr": null,
        "curr": "USD"
      },
      {
        "id": 9613,
        "ledger_id": 1594,
        "tx": 7117,
        "acct": 5737,
        "acct_name": "Income",
        "dr": null,
        "cr": "1000",
        "curr": "USD"
      }
    ]
  },
  {
    "id": 10099,
    "ledger_id": 1594,
    "posted": "2024-08-31T00:39:09.532222Z",
    "effective": "2024-08-31T00:39:09.532222Z",
    "memo": "client2",
    "meta": null,
    "entries": [
      {
        "id": 11045,
        "ledger_id": 1594,
        "tx": 10099,
        "acct": 2723,
        "acct_name": "Asset",
        "dr": "1500",
        "cr": null,
        "curr": "USD"
      },
      {
        "id": 12866,
        "ledger_id": 1594,
        "tx": 10099,
        "acct": 5737,
        "acct_name": "Income",
        "dr": null,
        "cr": "1500",
        "curr": "USD"
      }
    ]
  },
  {
    "id": 13440,
    "ledger_id": 1594,
    "posted": "2024-08-31T00:39:09.532222Z",
    "effective": "2024-08-31T00:39:09.532222Z",
    "memo": "lunch",
    "meta": null,
    "entries": [
      {
        "id": 14314,
        "ledger_id": 1594,
        "tx": 13440,
        "acct": 3720,
        "acct_name": "Expense",
        "dr": "20",
        "cr": null,
        "curr": "CAD"
      },
      {
        "id": 15828,
        "ledger_id": 1594,
        "tx": 13440,
        "acct": 2723,
        "acct_name": "Asset",
        "dr": null,
        "cr": "15",
        "curr": "USD"
      },
      {
        "id": 16268,
        "ledger_id": 1594,
        "tx": 13440,
        "acct": 6786,
        "acct_name": "Equity",
        "dr": "15",
        "cr": null,
        "curr": "USD"
      },
      {
        "id": 17899,
        "ledger_id": 1594,
        "tx": 13440,
        "acct": 6786,
        "acct_name": "Equity",
        "dr": null,
        "cr": "20",
        "curr": "CAD"
      }
    ]
  },
  {
    "id": 18592,
    "ledger_id": 1594,
    "posted": "2024-08-31T00:39:09.532222Z",
    "effective": "2024-08-31T00:39:09.532222Z",
    "memo": "dinner",
    "meta": null,
    "entries": [
      {
        "id": 19439,
        "ledger_id": 1594,
        "tx": 18592,
        "acct": 3720,
        "acct_name": "Expense",
        "dr": "28",
        "cr": null,
        "curr": "CAD"
      },
      {
        "id": 20416,
        "ledger_id": 1594,
        "tx": 18592,
        "acct": 2723,
        "acct_name": "Asset",
        "dr": null,
        "cr": "21",
        "curr": "USD"
      },
      {
        "id": 21671,
        "ledger_id": 1594,
        "tx": 18592,
        "acct": 6786,
        "acct_name": "Equity",
        "dr": "21",
        "cr": null,
        "curr": "USD"
      },
      {
        "id": 22327,
        "ledger_id": 1594,
        "tx": 18592,
        "acct": 6786,
        "acct_name": "Equity",
        "dr": null,
        "cr": "28",
        "curr": "CAD"
      }
    ]
  },
  {
    "id": 23421,
    "ledger_id": 1594,
    "posted": "2024-08-31T00:39:09.532222Z",
    "effective": "2024-08-31T00:39:09.532222Z",
    "memo": "5 MSFT @ 377.43 USD",
    "meta": null,
    "entries": [
      {
        "id": 24845,
        "ledger_id": 1594,
        "tx": 23421,
        "acct": 2723,
        "acct_name": "Asset",
        "dr": "5",
        "cr": null,
        "curr": "MSFT"
      },
      {
        "id": 25526,
        "ledger_id": 1594,
        "tx": 23421,
        "acct": 2723,
        "acct_name": "Asset",
        "dr": null,
        "cr": "1887",
        "curr": "USD"
      },
      {
        "id": 26281,
        "ledger_id": 1594,
        "tx": 23421,
        "acct": 6786,
        "acct_name": "Equity",
        "dr": null,
        "cr": "5",
        "curr": "MSFT"
      },
      {
        "id": 27335,
        "ledger_id": 1594,
        "tx": 23421,
        "acct": 6786,
        "acct_name": "Equity",
        "dr": "1887",
        "cr": null,
        "curr": "USD"
      }
    ]
  },
  {
    "id": 28755,
    "ledger_id": 1594,
    "posted": "2024-08-31T15:49:56.261779Z",
    "effective": "2024-08-31T15:49:56.261779Z",
    "memo": "test",
    "meta": null,
    "entries": [
      {
        "id": 29257,
        "ledger_id": 1594,
        "tx": 28755,
        "acct": 2723,
        "acct_name": "Asset",
        "dr": "1",
        "cr": null,
        "curr": "USD"
      },
      {
        "id": 30823,
        "ledger_id": 1594,
        "tx": 28755,
        "acct": 5737,
        "acct_name": "Income",
        "dr": null,
        "cr": "1",
        "curr": "USD"
      }
    ]
  }
]
</script>