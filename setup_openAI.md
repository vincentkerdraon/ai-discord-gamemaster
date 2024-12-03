# Setup OpenAI

Code assumes that assistant + thread + run already exists

## Assistant
Create in playground.

Validate: https://platform.openai.com/docs/api-reference/assistants/getAssistant
```bash
curl https://api.openai.com/v1/assistants/asst_abc123 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "OpenAI-Beta: assistants=v2"
```

## Thread + Run

Create: https://platform.openai.com/docs/api-reference/runs/createThreadAndRun (shortcut, or create separately)
```bash
curl https://api.openai.com/v1/threads/runs \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -H "OpenAI-Beta: assistants=v2" \
  -d '{
      "assistant_id": "asst_abc123",
      "thread": {
        "messages": [
          {"role": "user", "content": "Explain deep learning to a 5 year old."}
        ]
      }
    }'
```

Validate: https://platform.openai.com/docs/api-reference/messages/listMessages
```bash
curl https://api.openai.com/v1/threads/thread_abc123/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "OpenAI-Beta: assistants=v2"
```

## Send new message

This is done by the program.

//FIXME

## Read last message

This is done by the program.

Test: https://platform.openai.com/docs/api-reference/messages/listMessages
```bash
curl https://api.openai.com/v1/threads/thread_abc123/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "OpenAI-Beta: assistants=v2"
```
