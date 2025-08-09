#!/bin/bash

# Simple script to query the indexed ForgeCode project

QUERY="$1"
NUM_RESULTS="${2:-10}"

if [ -z "$QUERY" ]; then
    echo "Usage: ./ask_forge.sh '<your question>' [num_results]"
    echo ""
    echo "Examples:"
    echo "  ./ask_forge.sh 'how does the indexing pipeline work?'"
    echo "  ./ask_forge.sh 'error handling patterns' 5"
    echo "  ./ask_forge.sh 'file watching implementation'"
    echo "  ./ask_forge.sh 'vector database integration'"
    echo "  ./ask_forge.sh 'embedder types'"
    exit 1
fi

echo "🤖 Asking ForgeCode: $QUERY"
echo ""

# Use curl to query the API
curl -s -X POST http://localhost:8080/retrieve \
  -H "Content-Type: application/json" \
  -d "{
    \"query\": \"$QUERY\",
    \"repo\": \"forge-code\",
    \"branch\": \"main\",
    \"user_id\": \"local_user\",
    \"file_hashes\": {
      \"Cargo.toml\": \"dummy_hash\"
    },
    \"k\": $NUM_RESULTS
  }" | python3 -c "
import json
import sys

try:
    data = json.load(sys.stdin)
    
    if 'error' in data:
        print(f'❌ Error: {data[\"error\"]}')
        sys.exit(1)
    
    print(f'✅ Found {data[\"total_found\"]} results in {data[\"processing_time_ms\"]}ms')
    print(f'🆔 Request ID: {data[\"request_id\"]}')
    print('\\n' + '='*80)
    
    for i, chunk in enumerate(data['chunks'], 1):
        print(f'\\n📄 Result {i} - {chunk[\"path\"]} (Score: {chunk[\"score\"]:.4f})')
        print('-' * 60)
        print(chunk['code'])
        print('-' * 60)
        
    if not data['chunks']:
        print('🤷 No results found. The indexer might still be processing files.')
        
except json.JSONDecodeError:
    print('❌ Invalid response from API')
except KeyError as e:
    print(f'❌ Missing field in response: {e}')
except Exception as e:
    print(f'❌ Error: {e}')
"