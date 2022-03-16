This is a dumb program that just computes recursive disk usage and summarizes by owning UID.

It could be replaced by something like

```
find $PATH -type f -printf '%U %s\n' | awk '{ sizes[$1] +=  $2 } END { for (user in sizes) { print sizes[user] "\t" user } }' | sort -n
```
