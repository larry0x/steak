# Steak 🥩

Terra liquid staking derivative. Of the community, by the community, for the community.

Steak is audited by [SCV Security](https://twitter.com/TerraSCV) ([link](https://github.com/SCV-Security/PublicReports/blob/main/CW/St4k3h0us3/St4k3h0us3%20-%20Steak%20Contracts%20Audit%20Review%20-%20%20v1.0.pdf)).

## Contracts

| Contract                             | Description                                            |
| ------------------------------------ | ------------------------------------------------------ |
| [`steak-hub`](./contracts/hub)       | Manages minting/burning of Steak token and bonded Luna |
| [`steak-token`](./contracts/token)   | Modified CW20 token contract                           |
| [`steak-zapper`](./contracts/zapper) | Allows zapping into Steak from various assets          |

## Deployment

### Mainnet

| Contract             | Address                                        |
| -------------------- | ---------------------------------------------- |
| Steak Hub            | `terra15qr8ev2c0a0jswjtfrhfaj5ucgkhjd7la2shlg` |
| Steak Token          | `terra1rl4zyexjphwgx6v3ytyljkkc4mrje2pyznaclv` |
| Steak Zapper         | TBD                                            |
| Steak Admin Multisig | TBD                                            |

### Testnet

| Contract             | Address                                        |
| -------------------- | ---------------------------------------------- |
| Steak Hub            | `terra1xshrfs3lp7nwkdfh3067vfsf3kmweygfsc3hzy` |
| Steak Token          | `terra1awhvtkm553rszxtvnuda4fe2r6rjjj7hjwzv0w` |
| Steak Zapper         | TBD                                            |
| Steak Admin Multisig | TBD                                            |

## License

Contents of this repository are open source under [GNU General Public License v3](./LICENSE) or later.
