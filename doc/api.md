# Nimble Agent API

The Nimble agent (`nimbled`) provides a REST API on port `7080` by default.

## Endpoints

### List builds

`GET /builds`

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `status` | string | Filter by status (`queued`, `building`, `success`, `failed`) |
| `limit` | integer | Maximum number of builds to return |

**Example:**

```bash
curl http://localhost:7080/builds?status=queued&limit=10
```

**Response:** `200 OK`

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "success",
    "created_at": "2024-01-15 10:30:00",
    "updated_at": "2024-01-15 10:35:00"
  }
]
```

---

### Create a build

`POST /builds`

**Request Body:** Gzipped tar archive (`.tar.gz`) containing project source code.

**Example:**

```bash
curl -X POST http://localhost:7080/builds \
  -H "Content-Type: application/gzip" \
  --data-binary @project.tar.gz
```

**Response:** `200 OK`

```json
{
  "build_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "queued"
}
```

---

### Get build info

`GET /builds/:id`

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | UUID | Build identifier |

**Example:**

```bash
curl http://localhost:7080/builds/550e8400-e29b-41d4-a716-446655440000
```

**Response:** `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "success",
  "created_at": "2024-01-15 10:30:00",
  "updated_at": "2024-01-15 10:35:00"
}
```
