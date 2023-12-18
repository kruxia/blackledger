view.html

## newTransaction - expected behavior

- Add amount to first entry => a second entry is created to balance that entry.
    ```js
    {key:1, acct:1, dr:100, cr:null, curr:null, ref:null}       // dr:100
    {key:2, acct:null, dr:null, cr:100, curr:null, ref:1}
    ```
- Change the currency of the first entry => change the currency of the balancing entry.
    ```js
    {key:1, acct:1, dr:100, cr:null, curr:"USD", ref:null}      // curr:"USD"
    {key:2, acct:null, dr:null, cr:100, curr:"USD", ref:1}
    ```
- Change the amount of the first entry => change the balancing entry amount.
    ```js
    {key:1, acct:1, dr:200, cr:null, curr:"USD", ref:null}      // dr:200
    {key:2, acct:null, dr:null, cr:200, curr:"USD", ref:1}
    ```
- Type in debit => credit is cleared
    ```js
    {key:1, acct:1, dr:200, cr:null, curr:"USD", ref:null}
    {key:2, acct:null, dr:100, cr:null, curr:"USD", ref:1}      // dr:100
    {key:3, acct:null, dr:null, cr:300, curr:"USD", ref:2}
    ```
- Type in credit => debit is cleared
    ```js
    {key:1, acct:1, dr:200, cr:null, curr:"USD", ref:null}
    {key:2, acct:null, dr:null, cr:100, curr:"USD", ref:1}      // cr:100
    {key:3, acct:null, dr:null, cr:100, curr:"USD", ref:1}
    ```
- Change the Currency of an entry
    - => change currency of balancing entries
    - => create needed balancing entries
    ```js
    {key:1, acct:1, dr:200, cr:null, curr:"USD", ref:null}
    {key:2, acct:null, dr:null, cr:100, curr:"CAD", ref:1}      // curr:"CAD"
    {key:3, acct:null, dr:null, cr:100, curr:"USD", ref:1}
    {key:4, acct:null, dr:null, cr:100, curr:"USD", ref:1}
    {key:5, acct:null, dr:100, cr:null, curr:"CAD", ref:2}
    ```
- Remove the amount from an entry => remove balancing entries
    ```js
    {key:1, acct:1, dr:200, cr:null, curr:"USD", ref:null}
    {key:2, acct:null, dr:null, cr:null, curr:"CAD", ref:1}     // cr:null
    {key:3, acct:null, dr:null, cr:100, curr:"USD", ref:1}
    {key:4, acct:null, dr:null, cr:100, curr:"USD", ref:1}
    ```

- Remove the currency from an entry with no amounts => remove entry
    ```js
    {key:1, acct:1, dr:200, cr:null, curr:"USD", ref:null}
    // {key:2, acct:null, dr:null, cr:null, curr:null, ref:1}   // curr:null
    {key:3, acct:null, dr:null, cr:100, curr:"USD", ref:1}
    {key:4, acct:null, dr:null, cr:100, curr:"USD", ref:1}
    ```
