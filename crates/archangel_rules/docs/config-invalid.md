# config-invalid

The config has a field archangel does not know, a value of the wrong type, or text that is not valid JSON. The diagnostic names the field, such as `rules[0].reasn`.

## Example

```json
{ "files": "src/**", "noCycles": true, "reasn": "typo" }
```

## What to do

Fix the field the message names. Every problem in the config is reported at once.
