# OpenAI

## Setup OpenAI

Code assumes that assistant + thread already exists.
Create manually if needed.

### Assistant
Create in playground.

Validate: https://platform.openai.com/docs/api-reference/assistants/getAssistant
```bash
 export OPENAI_API_KEY=sk-abc
export ASSISTANT_ID=asst_uQJ4xO4Rx1HvVhajwkGJoOzj
curl https://api.openai.com/v1/assistants/$ASSISTANT_ID \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "OpenAI-Beta: assistants=v2"
```

### Thread + Run

Create: https://platform.openai.com/docs/api-reference/runs/createThreadAndRun (shortcut, or create separately)
```bash
curl https://api.openai.com/v1/threads/runs \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -H "OpenAI-Beta: assistants=v2" \
  -d '{
      "assistant_id": "$ASSISTANT_ID",
      "thread": {
        "messages": [
          {"role": "user", "content": "Explain deep learning to a 5 year old."}
        ]
      }
    }'
```

Validate: https://platform.openai.com/docs/api-reference/messages/listMessages
```bash
export THREAD_ID="thread_Ur2NyQm7FdM2ggbl7MF12PSg"
curl https://api.openai.com/v1/threads/$THREAD_ID/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "OpenAI-Beta: assistants=v2"
```

## Call OpenAI from the program

This is done by the program. 

### Send new message

https://platform.openai.com/docs/api-reference/messages/createMessage
```bash
curl https://api.openai.com/v1/threads/$THREAD_ID/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "OpenAI-Beta: assistants=v2" \
  -d '{
      "role": "user",
      "content": "What is your last order?"
    }'
```

### Start run

https://platform.openai.com/docs/api-reference/runs/createRun
```bash
curl https://api.openai.com/v1/threads/$THREAD_ID/runs \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -H "OpenAI-Beta: assistants=v2" \
  -d "{
    \"assistant_id\": \"$ASSISTANT_ID\"
  }"
```

### Wait for run completion and get messageId

https://platform.openai.com/docs/api-reference/run-steps/getRunStep
```bash
export RUN_ID=run_AYkRsqL2C4OlLxxjDeP5B4cF
curl https://api.openai.com/v1/threads/$THREAD_ID/runs/$RUN_ID/steps \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -H "OpenAI-Beta: assistants=v2"
```


curl https://api.openai.com/v1/threads/thread_Ur2NyQm7FdM2ggbl7MF12PSg/runs/run_89fH7sJUytUCKo51cLkZbBtt/steps \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -H "OpenAI-Beta: assistants=v2"

### Read message

https://platform.openai.com/docs/api-reference/messages/getMessage
```bash
export MESSAGE_ID=msg_OPNBFBdrhXVCZyBx2VS2fqCG
curl https://api.openai.com/v1/threads/$THREAD_ID/messages/$MESSAGE_ID \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "OpenAI-Beta: assistants=v2"
```

### Generate audio

https://platform.openai.com/docs/api-reference/audio/createSpeech
```bash
curl https://api.openai.com/v1/audio/speech \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "tts-1",
    "input": "Today is a wonderful day to build something people love!",
    "voice": "onyx",
    "speed": "1.4"
  }' \
  --output speech.mp3
```