# Documentation Development

## Development

Install the Mintlify CLI to preview your documentation changes locally:

```
npm i -g mint
```

Run the following command at the root of your documentation, where your `docs.json` is located:

```
mint dev
```

View your local preview at `http://localhost:3000`.

## Publishing changes

Documentation changes for this fork are reviewed and merged through the main repository workflow. Keep links and references aligned with `davidrudduck/vk-swarm`.

## Need help?

### Troubleshooting

- If your dev environment isn't running: Run `mint update` to ensure you have the most recent version of the CLI.
- If a page loads as a 404: Make sure you are running in a folder with a valid `docs.json`.

### Resources
- [Repository root README](../README.md)
- [Architecture docs](./architecture/README.md)
