# Steak 🥩

Terra liquid staking derivative. Of the community, by the community, for the community.

A previous version ([v1.0.0-rc0](https://github.com/st4k3h0us3/steak-contracts/releases/tag/v1.0.0-rc0)) of Steak was audited by [SCV Security](https://twitter.com/TerraSCV) ([link](https://github.com/SCV-Security/PublicReports/blob/main/CW/St4k3h0us3/St4k3h0us3%20-%20Steak%20Contracts%20Audit%20Review%20-%20%20v1.0.pdf)).

## Contracts

| Contract                           | Description                                            |
| ---------------------------------- | ------------------------------------------------------ |
| [`steak-hub`](./contracts/hub)     | Manages minting/burning of Steak token and bonded Luna |
| [`steak-token`](./contracts/token) | Modified CW20 token contract                           |

## Deployment

### Mainnet (phoenix-1)

| Contract            | Address                                                                                                                                                                           |
| ------------------- |-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Steak Hub           | [`terra12e4v50xl33fnwkzltz9vu565snlmx65vdrk8e2644km09myewr8q538psc`](https://finder.terra.money/mainnet/address/terra12e4v50xl33fnwkzltz9vu565snlmx65vdrk8e2644km09myewr8q538psc) |
| Steak Token         | [`terra1xumzh893lfa7ak5qvpwmnle5m5xp47t3suwwa9s0ydqa8d8s5faqn6x7al`](https://finder.terra.money/mainnet/address/terra1xumzh893lfa7ak5qvpwmnle5m5xp47t3suwwa9s0ydqa8d8s5faqn6x7al) |
| STEAK-LUNA Pair     | [`terra1jynmf6gteg4rd03ztldan5j2dp78su4tc3hfvkve8dl068c2yppsk5uszc`](https://finder.terra.money/mainnet/address/terra1jynmf6gteg4rd03ztldan5j2dp78su4tc3hfvkve8dl068c2yppsk5uszc)                                                                                                           |
| STEAK-LUNA LP Token | [`tbd`]()                                                                                                                                                                         |

### Testnet (pisco-1)

| Contract            | Address                                                                                                                                                                           |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Steak Hub           | [`terra1lm7d4zr97rzp3a22szdv6ucpeyckyl2l2wh6jc9qrga78eyrvamsjgs5q6`](https://finder.terra.money/testnet/address/terra1lm7d4zr97rzp3a22szdv6ucpeyckyl2l2wh6jc9qrga78eyrvamsjgs5q6) |
| Steak Token         | [`terra1q02glqy2gcl9kavshhh9tzr3l7cjwjwk7mhatd9h9nc243gq73esdat6wj`](https://finder.terra.money/testnet/address/terra1q02glqy2gcl9kavshhh9tzr3l7cjwjwk7mhatd9h9nc243gq73esdat6wj) |
| STEAK-LUNA Pair     | [`tbd`]()                                                                                                                                                                         |
| STEAK-LUNA LP Token | [`tbd`]()                                                                                                                                                                         |

### Classic mainnet (columbus-5)

| Contract            | Address                                                                                                                                   |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Steak Hub           | [`terra15qr8ev2c0a0jswjtfrhfaj5ucgkhjd7la2shlg`](https://finder.terra.money/classic/address/terra15qr8ev2c0a0jswjtfrhfaj5ucgkhjd7la2shlg) |
| Steak Token         | [`terra1rl4zyexjphwgx6v3ytyljkkc4mrje2pyznaclv`](https://finder.terra.money/classic/address/terra1rl4zyexjphwgx6v3ytyljkkc4mrje2pyznaclv) |
| STEAK-LUNA Pair     | [`terra14q0cgunptuym048a4y2awt8a7fl9acudmfzk5e`](https://finder.terra.money/classic/address/terra14q0cgunptuym048a4y2awt8a7fl9acudmfzk5e) |
| STEAK-LUNA LP Token | [`terra1pwc77c6a588cualln2uypyyvg5r76tfaazgk62`](https://finder.terra.money/classic/address/terra1pwc77c6a588cualln2uypyyvg5r76tfaazgk62) |

### Classic testnet (bombay-12)

| Contract            | Address                                        |
| ------------------- | ---------------------------------------------- |
| Steak Hub           | `terra1xshrfs3lp7nwkdfh3067vfsf3kmweygfsc3hzy` |
| Steak Token         | `terra1awhvtkm553rszxtvnuda4fe2r6rjjj7hjwzv0w` |
| STEAK-LUNA Pair     | `terra1x3tyfme7y84mv3y6ftugllrln5y7ewhf36davz` |
| STEAK-LUNA LP Token | `terra1exla7lyc8g85szpntmcs5f2rvvg5gwwn7jekje` |

## License

Contents of this repository are open source under [GNU General Public License v3](./LICENSE) or later.
