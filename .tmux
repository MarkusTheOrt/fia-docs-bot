#!/bin/bash

# Name of the tmux session
SESSION_NAME="$(basename "$PWD")"

# Check if the session already exists
tmux has-session -t $SESSION_NAME 2>/dev/null
if [ $? != 0 ]; then
  # Create a new tmux session but don't attach yet
  tmux new-session -d -s $SESSION_NAME
fi
  # Create the first window, set path, and run a program
  tmux rename-window -t $SESSION_NAME:0 "nvim"
  tmux send-keys -t $SESSION_NAME:0 "clear && nvim ." C-m

  # Create the second window, set path, and run a program
  tmux new-window -t $SESSION_NAME:1 -n "compose"
  tmux send-keys -t $SESSION_NAME:1 "clear && docker-compose -f ./compose.dev.yaml up" C-m

  tmux new-window -t $SESSION_NAME:2 -n "bot"
  tmux send-keys -t $SESSION_NAME:2 "clear && bacon run" C-m


  # Select the first window to be active when attaching
  tmux select-window -t $SESSION_NAME:0

# Attach to the session
tmux attach -t $SESSION_NAME
