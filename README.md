This is the voting server for the Coin Voting
system used in the Devfund and NSM polls[^1].

It can also be used to *verify ballots*.

## Verification

> You must have the ballot id. That's the hash
value that was shown on the screen after the submission.

1. Download the ballot from the server
1. Verify that the ballot is correct and show the candidate
& amount

### Download from the server

Use a REST query (either through the browser, curl, wget,
postman, ...)

The endpoint is `<server_url>/ballot/<hash>`

Save the result to a file.

### Verify the ballot data

- Install rust
- Build and run the vote-server app with `cargo r -r`

On its command line, 
```
validate <election id> <ballot file name>
```

The election id is in the election json file that
can be retrieved from the vote url.

## Testing

The NSM voting server is running at `https://vote.zcash-infra.com/nsm-nu7/`

---
[^1]: There are some refactoring but the logic
remains the same. Ballots submitted during the
previous polls are compatible.

