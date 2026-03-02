# Bridge Registry

A collection of all planned to be supported bridges with simple enum and possible a simple sdk to pull data about each one of them (supported tokens/chains)

## Hyperlane

Hyperlane fetches data from the [canonical registry](https://github.com/hyperlane-xyz/hyperlane-registry) via GitHub API. For `tokens` (many API calls), set `GITHUB_TOKEN` to avoid rate limits (60/hr unauthenticated, 5000/hr with token).

Commands:

- chains (bridge -> chains) - list supported chains by bridge
- tokens (bridge -> tokens) - list all supported tokens by bridge
- list (None -> bridge names) - list all known bridges
- bridges - list all bridges that support given chainID
- help - list all bridges that support given chainID

Input:

- Bridge Name (ID), eg. stargate, hyperlane, axelar, wormhole, etc.
- chains or tokens
- Unmodified flag -

Output:

- JSON output of chains or tokens that bridge support

## Specification

Specs from [UNIX Lopata README](../README.md) MUST be followed for unification and to achieve global goal
