# Blackledger

Blackledger is a powerful financial management system accessible via HTTP REST API endpoints. It leverages double-entry accounting principles, ensuring precise and accurate transaction handling. This system enables users to manage finances with decimal precision, account versioning for optimistic locking, support for multiple currencies, immutable transaction entries, and effective date and posting date mechanisms for maintaining historical accuracy.

## Try it out

```sh
docker compose up -d
docker compose logs -f
```

Then browse to <http://localhost:8000/docs> and try out the API.

![API Docs running locally](https://github.com/user-attachments/assets/32ac70c1-27e8-4047-99eb-05510e4f944e)
