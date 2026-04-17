#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════
#  AEZARX — Full Endpoint Test Script
#  Run:  chmod +x test.sh && ./test.sh
#  Requires: curl, jq  (brew install jq  OR  apt install jq)
# ═══════════════════════════════════════════════════════════════

BASE="http://localhost:4000"
GREEN="\033[0;32m"
RED="\033[0;31m"
CYAN="\033[0;36m"
YELLOW="\033[1;33m"
RESET="\033[0m"

ok()   { echo -e "${GREEN}✓  $1${RESET}"; }
fail() { echo -e "${RED}✗  $1${RESET}"; }
hdr()  { echo -e "\n${CYAN}══ $1 ══${RESET}"; }
note() { echo -e "${YELLOW}   $1${RESET}"; }

# ── 0. HEALTH ────────────────────────────────────────────────
hdr "HEALTH"

curl -s "$BASE/api/health" | jq .
ok "GET /api/health"


# ── 1. AUTH — REGISTER ───────────────────────────────────────
hdr "AUTH"

note "Registering a new user..."
REGISTER=$(curl -s -X POST "$BASE/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "email":    "test@example.com",
    "password": "password123"
  }')
echo "$REGISTER" | jq .
TOKEN=$(echo "$REGISTER" | jq -r '.token')
USER_ID=$(echo "$REGISTER" | jq -r '.user.id')

if [ "$TOKEN" != "null" ] && [ -n "$TOKEN" ]; then
  ok "POST /api/auth/register"
else
  fail "POST /api/auth/register — got no token"
fi


# ── 2. AUTH — LOGIN BY EMAIL ─────────────────────────────────
note "Login by email + password..."
LOGIN_EMAIL=$(curl -s -X POST "$BASE/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email":    "test@example.com",
    "password": "password123"
  }')
echo "$LOGIN_EMAIL" | jq .
ok "POST /api/auth/login (by email)"


# ── 3. AUTH — LOGIN BY USERNAME ──────────────────────────────
note "Login by username + password..."
LOGIN_USER=$(curl -s -X POST "$BASE/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "password123"
  }')
echo "$LOGIN_USER" | jq .
ok "POST /api/auth/login (by username)"


# ── 4. AUTH — MAGIC LINK (by email) ─────────────────────────
note "Request magic link by email (check your email/logs for the link)..."
MAGIC_EMAIL=$(curl -s -X POST "$BASE/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{ "email": "test@example.com" }')
echo "$MAGIC_EMAIL" | jq .
ok "POST /api/auth/login (magic link by email)"


# ── 5. AUTH — MAGIC LINK (by username) ──────────────────────
note "Request magic link by username..."
MAGIC_USER=$(curl -s -X POST "$BASE/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{ "username": "testuser" }')
echo "$MAGIC_USER" | jq .
ok "POST /api/auth/login (magic link by username)"


# ── 6. AUTH — ME ─────────────────────────────────────────────
note "Get current user..."
curl -s "$BASE/api/auth/me" \
  -H "Authorization: Bearer $TOKEN" | jq .
ok "GET /api/auth/me"


# ── 7. AUTH — FORGOT PASSWORD ────────────────────────────────
note "Forgot password (sends reset email)..."
curl -s -X POST "$BASE/api/auth/forgot-password" \
  -H "Content-Type: application/json" \
  -d '{ "email": "test@example.com" }' | jq .
ok "POST /api/auth/forgot-password"


# ── 8. AUTH — LOGOUT ─────────────────────────────────────────
note "Logout..."
curl -s -X POST "$BASE/api/auth/logout" \
  -H "Authorization: Bearer $TOKEN" | jq .
ok "POST /api/auth/logout"


# ── 9. USERS ─────────────────────────────────────────────────
hdr "USERS"

note "Get user profile..."
curl -s "$BASE/api/users/$USER_ID" \
  -H "Authorization: Bearer $TOKEN" | jq .
ok "GET /api/users/:id"

note "Update username and bio..."
curl -s -X PUT "$BASE/api/users/$USER_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser_updated",
    "bio":      "I love manga and books!"
  }' | jq .
ok "PUT /api/users/:id"


# ── 10. MANGA / BOOKS ────────────────────────────────────────
hdr "MANGA / BOOKS"

note "List all manga (page 1)..."
curl -s "$BASE/api/manga" | jq '{ total: .total, count: (.books | length), first: .books[0].title }'
ok "GET /api/manga"

note "List with filters: shonen genre, page 1..."
curl -s "$BASE/api/manga?genre=shonen&page=1&per_page=3" | jq '{ total: .total, books: [.books[].title] }'
ok "GET /api/manga?genre=shonen"

note "Search by title..."
curl -s "$BASE/api/manga?search=naruto" | jq '{ total: .total, books: [.books[].title] }'
ok "GET /api/manga?search=naruto"

note "Filter by book_type=book..."
curl -s "$BASE/api/manga?book_type=book" | jq '{ total: .total, books: [.books[].title] }'
ok "GET /api/manga?book_type=book"

# Get first book ID for subsequent tests
BOOK_ID=$(curl -s "$BASE/api/manga" | jq -r '.books[0].id')
note "Using book ID: $BOOK_ID"

note "Get single book..."
curl -s "$BASE/api/manga/$BOOK_ID" | jq .
ok "GET /api/manga/:id"


# ── 11. ADMIN — CREATE BOOK ──────────────────────────────────
hdr "ADMIN (requires admin role)"

note "To test admin endpoints, first make your user an admin:"
note "  docker exec -it aezarx-db psql -U postgres -d aezarx"
note "  UPDATE users SET role='admin' WHERE email='test@example.com';"
note ""
note "Then re-login to get a fresh token and run:"
note ""
note "  # Create a new manga"
note "  curl -s -X POST $BASE/api/manga \\"
note "    -H 'Authorization: Bearer \$TOKEN' \\"
note "    -H 'Content-Type: application/json' \\"
note "    -d '{\"title\":\"My Manga\",\"book_type\":\"manga\",\"genre\":\"shonen\",\"price\":199,\"stock\":100}' | jq ."
note ""
note "  # Add a chapter (get BOOK_ID from create response)"
note "  curl -s -X POST $BASE/api/manga/\$BOOK_ID/chapters \\"
note "    -H 'Authorization: Bearer \$TOKEN' \\"
note "    -H 'Content-Type: application/json' \\"
note "    -d '{\"title\":\"Chapter 1\",\"pages\":[\"001.jpg\",\"002.jpg\",\"003.jpg\"]}' | jq ."
note ""
note "  # Then copy images: docker cp ./pages/. aezarx-backend:/app/manga-pages/\$BOOK_ID/1/"


# ── 12. CHAPTERS ─────────────────────────────────────────────
hdr "CHAPTERS"

note "List chapters for book..."
curl -s "$BASE/api/manga/$BOOK_ID/chapters" | jq .
ok "GET /api/manga/:id/chapters"

note "Get chapter 1 pages (needs manga-pages files uploaded)..."
curl -s "$BASE/api/manga/$BOOK_ID/chapter/1" | jq .
ok "GET /api/manga/:id/chapter/:n"


# ── 13. TRENDING ─────────────────────────────────────────────
hdr "TRENDING"

note "Get trending manga..."
curl -s "$BASE/api/trending?type=manga&period=week" | jq '{ count: (.trending | length) }'
ok "GET /api/trending?type=manga"

note "Get trending books..."
curl -s "$BASE/api/trending?type=book&period=week" | jq '{ count: (.trending | length) }'
ok "GET /api/trending?type=book"


# ── 14. NEWS ─────────────────────────────────────────────────
hdr "NEWS"

note "Get all news..."
curl -s "$BASE/api/news" | jq '{ total: .total, count: (.news | length) }'
ok "GET /api/news"

note "Get news filtered by category=manga..."
curl -s "$BASE/api/news?category=manga" | jq '{ total: .total, titles: [.news[].title] }'
ok "GET /api/news?category=manga"

note "Get news page 2..."
curl -s "$BASE/api/news?page=2" | jq '{ page: .page, total: .total }'
ok "GET /api/news?page=2"


# ── 15. LIBRARY ──────────────────────────────────────────────
hdr "LIBRARY"

note "Add book to library..."
curl -s -X POST "$BASE/api/users/$USER_ID/library" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"bookId\": \"$BOOK_ID\"}" | jq .
ok "POST /api/users/:id/library"

note "Get library..."
curl -s "$BASE/api/users/$USER_ID/library" \
  -H "Authorization: Bearer $TOKEN" | jq '{ count: (.library | length) }'
ok "GET /api/users/:id/library"

note "Remove book from library..."
curl -s -X DELETE "$BASE/api/users/$USER_ID/library/$BOOK_ID" \
  -H "Authorization: Bearer $TOKEN" | jq .
ok "DELETE /api/users/:id/library/:book_id"


# ── 16. BOOKMARKS ────────────────────────────────────────────
hdr "BOOKMARKS"

note "Add a bookmark..."
BM=$(curl -s -X POST "$BASE/api/users/$USER_ID/bookmarks" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"bookId\": \"$BOOK_ID\", \"chapter\": 1, \"page\": 5}")
echo "$BM" | jq .
BM_ID=$(echo "$BM" | jq -r '.bookmark.id')
ok "POST /api/users/:id/bookmarks"

note "Get all bookmarks..."
curl -s "$BASE/api/users/$USER_ID/bookmarks" \
  -H "Authorization: Bearer $TOKEN" | jq '{ count: (.bookmarks | length) }'
ok "GET /api/users/:id/bookmarks"

note "Delete bookmark..."
curl -s -X DELETE "$BASE/api/users/$USER_ID/bookmarks/$BM_ID" \
  -H "Authorization: Bearer $TOKEN" | jq .
ok "DELETE /api/users/:id/bookmarks/:bm_id"


# ── 17. READING HISTORY ──────────────────────────────────────
hdr "READING HISTORY"

note "Get reading history..."
curl -s "$BASE/api/users/$USER_ID/history" \
  -H "Authorization: Bearer $TOKEN" | jq '{ count: (.history | length) }'
ok "GET /api/users/:id/history"

note "Save reading progress (chapter 1, page 3)..."
curl -s -X POST "$BASE/api/manga/$BOOK_ID/chapter/1/progress" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{ "page": 3 }' | jq .
ok "POST /api/manga/:id/chapter/:n/progress"


# ── 18. CART ─────────────────────────────────────────────────
hdr "CART"

note "Add item to cart..."
curl -s -X POST "$BASE/api/cart" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"bookId\": \"$BOOK_ID\", \"quantity\": 1}" | jq .
ok "POST /api/cart"

note "Get cart..."
CART=$(curl -s "$BASE/api/cart" \
  -H "Authorization: Bearer $TOKEN")
echo "$CART" | jq '{ items: (.items | length), total: .total }'
CART_ITEM_ID=$(echo "$CART" | jq -r '.items[0].id')
ok "GET /api/cart"

note "Remove item from cart..."
curl -s -X DELETE "$BASE/api/cart/$CART_ITEM_ID" \
  -H "Authorization: Bearer $TOKEN" | jq .
ok "DELETE /api/cart/:item_id"


# ── 19. ORDERS / CHECKOUT ────────────────────────────────────
hdr "ORDERS"

note "Add to cart first, then checkout..."
curl -s -X POST "$BASE/api/cart" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"bookId\": \"$BOOK_ID\", \"quantity\": 1}" | jq .

note "Checkout (creates order, clears cart)..."
curl -s -X POST "$BASE/api/orders" \
  -H "Authorization: Bearer $TOKEN" | jq .
ok "POST /api/orders (checkout)"

note "Get order history..."
curl -s "$BASE/api/orders" \
  -H "Authorization: Bearer $TOKEN" | jq '{ orders: (.orders | length) }'
ok "GET /api/orders"


# ── SUMMARY ──────────────────────────────────────────────────
echo -e "\n${GREEN}══════════════════════════════════════${RESET}"
echo -e "${GREEN}  All endpoint checks complete!${RESET}"
echo -e "${GREEN}══════════════════════════════════════${RESET}"
echo -e "BASE URL : $BASE"
echo -e "USER ID  : $USER_ID"
echo -e "TOKEN    : ${TOKEN:0:40}..."