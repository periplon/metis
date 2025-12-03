# DataFusion SQL Queries

Metis uses Apache DataFusion to provide SQL query capabilities over Data Lake records stored in Parquet or JSONL files.

## Basic Usage

In the Data Lake Records UI, navigate to the **Query** tab and enter your SQL query. Use `$table` as a placeholder for the table name - it will be automatically replaced with the registered table name.

### Simple Queries

```sql
-- Select all records
SELECT * FROM $table

-- Select specific columns
SELECT id, schema_name, created_at FROM $table

-- Limit results
SELECT * FROM $table LIMIT 10

-- Order by column
SELECT * FROM $table ORDER BY created_at DESC
```

## JSON Query Functions

The `data` column contains JSON data. Use these functions to query JSON content:

### Extract Values

| Function | Description | Example |
|----------|-------------|---------|
| `json_get(json, path...)` | Get value at path (returns JSON) | `json_get(data, 'user')` |
| `json_get_str(json, path...)` | Get string value at path | `json_get_str(data, 'name')` |
| `json_get_int(json, path...)` | Get integer value at path | `json_get_int(data, 'count')` |
| `json_get_float(json, path...)` | Get float value at path | `json_get_float(data, 'price')` |
| `json_get_bool(json, path...)` | Get boolean value at path | `json_get_bool(data, 'active')` |

### JSON Utilities

| Function | Description | Example |
|----------|-------------|---------|
| `json_as_text(json)` | Convert JSON to text | `json_as_text(data)` |
| `json_contains(json, value)` | Check if JSON contains value | `json_contains(data, '"active"')` |
| `json_keys(json)` | Get array of keys | `json_keys(data)` |
| `json_length(json)` | Get length of array/object | `json_length(json_get(data, 'items'))` |

## Query Examples

### Extract a Field from JSON

```sql
SELECT id, json_get_str(data, 'name') as name
FROM $table
```

### Filter by JSON Field Value

```sql
SELECT * FROM $table
WHERE json_get_str(data, 'status') = 'active'
```

### Extract Nested JSON Fields

```sql
-- Get nested field: data.user.email
SELECT id, json_get_str(data, 'user', 'email') as email
FROM $table
```

### Numeric Comparisons on JSON Fields

```sql
SELECT id, json_get_int(data, 'quantity') as qty
FROM $table
WHERE json_get_int(data, 'quantity') > 10
```

### Combine Multiple JSON Filters

```sql
SELECT
    id,
    json_get_str(data, 'name') as name,
    json_get_str(data, 'category') as category,
    json_get_float(data, 'price') as price
FROM $table
WHERE json_get_str(data, 'category') = 'electronics'
  AND json_get_float(data, 'price') < 100.00
ORDER BY price DESC
```

### Filter by Boolean Field

```sql
SELECT * FROM $table
WHERE json_get_bool(data, 'is_published') = true
```

### Check if Field Exists (is not null)

```sql
SELECT * FROM $table
WHERE json_get_str(data, 'email') IS NOT NULL
```

### Aggregate JSON Data

```sql
SELECT
    json_get_str(data, 'category') as category,
    COUNT(*) as count,
    AVG(json_get_float(data, 'price')) as avg_price
FROM $table
GROUP BY json_get_str(data, 'category')
```

### Pattern Matching on JSON Strings

```sql
SELECT * FROM $table
WHERE json_get_str(data, 'email') LIKE '%@gmail.com'
```

## Data Schema

Each record in a Data Lake table has these columns:

| Column | Type | Description |
|--------|------|-------------|
| `id` | String | Unique record identifier |
| `data_lake` | String | Name of the data lake |
| `schema_name` | String | Schema the record belongs to |
| `data` | String (JSON) | The record's JSON data |
| `created_at` | String | ISO 8601 timestamp |
| `updated_at` | String | ISO 8601 timestamp |
| `created_by` | String (nullable) | User who created the record |
| `metadata` | String (nullable) | Additional JSON metadata |

## Restrictions

For security, only `SELECT` queries are allowed. The following operations are blocked:
- `DROP`
- `DELETE`
- `TRUNCATE`
- `ALTER`
- `INSERT`
- `UPDATE`

## Table Naming and JOINs

Tables are registered with schema-based naming: `datalake_name.schema_name`. This allows JOINs between tables from different data lakes or schemas.

### Table Naming Convention

```
datalake_name.schema_name
```

For example, if you have:
- Data lake: `ecommerce`
- Schemas: `orders`, `customers`, `products`

The tables would be:
- `ecommerce.orders`
- `ecommerce.customers`
- `ecommerce.products`

### JOIN Examples

#### Join Two Schemas in the Same Data Lake

```sql
SELECT
    o.id as order_id,
    json_get_str(o.data, 'order_date') as order_date,
    json_get_str(c.data, 'name') as customer_name,
    json_get_str(c.data, 'email') as customer_email
FROM ecommerce.orders o
JOIN ecommerce.customers c
    ON json_get_str(o.data, 'customer_id') = c.id
```

#### Join Tables from Different Data Lakes

```sql
SELECT
    p.id as product_id,
    json_get_str(p.data, 'name') as product_name,
    json_get_float(i.data, 'quantity') as stock_quantity
FROM catalog.products p
JOIN inventory.stock i
    ON p.id = json_get_str(i.data, 'product_id')
```

#### Left Join with Aggregation

```sql
SELECT
    c.id as customer_id,
    json_get_str(c.data, 'name') as customer_name,
    COUNT(o.id) as order_count,
    SUM(json_get_float(o.data, 'total')) as total_spent
FROM ecommerce.customers c
LEFT JOIN ecommerce.orders o
    ON c.id = json_get_str(o.data, 'customer_id')
GROUP BY c.id, json_get_str(c.data, 'name')
```

#### Self-Join for Hierarchical Data

```sql
SELECT
    e.id as employee_id,
    json_get_str(e.data, 'name') as employee_name,
    json_get_str(m.data, 'name') as manager_name
FROM hr.employees e
LEFT JOIN hr.employees m
    ON json_get_str(e.data, 'manager_id') = m.id
```

### Using $table Placeholder

When using the Query tab in the UI, `$table` is replaced with the current data lake and schema. You can still reference other tables by their full name:

```sql
SELECT
    t.*,
    json_get_str(ref.data, 'description') as category_description
FROM $table t
JOIN catalog.categories ref
    ON json_get_str(t.data, 'category_id') = ref.id
```

## Performance Tips

1. **Use specific columns** instead of `SELECT *` when possible
2. **Filter early** - put WHERE clauses on indexed columns first
3. **Limit results** - use `LIMIT` to reduce data transfer
4. **Avoid complex JSON operations** on large datasets - extract frequently queried fields to top-level columns if needed
5. **Index JOIN keys** - when joining on JSON fields, consider extracting frequently-joined fields to top-level columns

## Aggregate Functions

DataFusion supports a rich set of aggregate functions for summarizing data:

| Function | Description |
|----------|-------------|
| `COUNT(*)`, `COUNT(col)` | Count rows or non-null values |
| `SUM(col)`, `AVG(col)` | Sum and average |
| `MIN(col)`, `MAX(col)` | Minimum and maximum |
| `STDDEV(col)`, `VARIANCE(col)` | Standard deviation and variance |
| `ARRAY_AGG(col)` | Collect values into array |
| `STRING_AGG(col, delimiter)` | Concatenate strings with delimiter |
| `BOOL_AND(col)`, `BOOL_OR(col)` | Boolean aggregates |
| `FIRST_VALUE(col)`, `LAST_VALUE(col)` | First/last value in group |
| `APPROX_DISTINCT(col)` | Approximate count distinct (HyperLogLog) |
| `APPROX_PERCENTILE_CONT(col, pct)` | Approximate percentile |

### Aggregate Examples

```sql
-- Multiple aggregations
SELECT
    json_get_str(data, 'category') as category,
    COUNT(*) as total,
    SUM(json_get_float(data, 'amount')) as total_amount,
    AVG(json_get_float(data, 'amount')) as avg_amount,
    MIN(json_get_float(data, 'amount')) as min_amount,
    MAX(json_get_float(data, 'amount')) as max_amount
FROM $table
GROUP BY json_get_str(data, 'category')

-- String aggregation
SELECT
    json_get_str(data, 'order_id') as order_id,
    STRING_AGG(json_get_str(data, 'product_name'), ', ') as products
FROM $table
GROUP BY json_get_str(data, 'order_id')

-- Approximate percentile
SELECT
    json_get_str(data, 'region') as region,
    APPROX_PERCENTILE_CONT(json_get_float(data, 'sales'), 0.5) as median_sales,
    APPROX_PERCENTILE_CONT(json_get_float(data, 'sales'), 0.95) as p95_sales
FROM $table
GROUP BY json_get_str(data, 'region')
```

## Window Functions

Window functions perform calculations across a set of rows related to the current row.

### Syntax

```sql
function_name() OVER (
    PARTITION BY col1, col2
    ORDER BY col3
    ROWS BETWEEN ... AND ...
)
```

### Available Window Functions

| Function | Description |
|----------|-------------|
| `ROW_NUMBER()` | Sequential row number within partition |
| `RANK()` | Rank with gaps for ties |
| `DENSE_RANK()` | Rank without gaps |
| `NTILE(n)` | Divide rows into n buckets |
| `LAG(col, n)` | Value from n rows before current row |
| `LEAD(col, n)` | Value from n rows after current row |
| `FIRST_VALUE(col)` | First value in the window frame |
| `LAST_VALUE(col)` | Last value in the window frame |
| `NTH_VALUE(col, n)` | Nth value in the window frame |
| `SUM/AVG/COUNT() OVER()` | Running/cumulative aggregates |

### Window Function Examples

```sql
-- Running total
SELECT
    id,
    json_get_float(data, 'amount') as amount,
    SUM(json_get_float(data, 'amount')) OVER (ORDER BY created_at) as running_total
FROM $table

-- Rank within category
SELECT
    id,
    json_get_str(data, 'category') as category,
    json_get_float(data, 'score') as score,
    RANK() OVER (
        PARTITION BY json_get_str(data, 'category')
        ORDER BY json_get_float(data, 'score') DESC
    ) as rank
FROM $table

-- Compare to previous row
SELECT
    id,
    json_get_float(data, 'value') as current_value,
    LAG(json_get_float(data, 'value'), 1) OVER (ORDER BY created_at) as prev_value,
    json_get_float(data, 'value') - LAG(json_get_float(data, 'value'), 1) OVER (ORDER BY created_at) as change
FROM $table

-- Moving average
SELECT
    id,
    created_at,
    json_get_float(data, 'value') as value,
    AVG(json_get_float(data, 'value')) OVER (
        ORDER BY created_at
        ROWS BETWEEN 6 PRECEDING AND CURRENT ROW
    ) as moving_avg_7
FROM $table

-- Top N per group
SELECT * FROM (
    SELECT
        id,
        json_get_str(data, 'category') as category,
        json_get_float(data, 'sales') as sales,
        ROW_NUMBER() OVER (
            PARTITION BY json_get_str(data, 'category')
            ORDER BY json_get_float(data, 'sales') DESC
        ) as rn
    FROM $table
) WHERE rn <= 3
```

## Process Analysis

DataFusion is well-suited for process mining and event log analysis.

### Event Sequencing

```sql
-- Event sequences with previous and next activities
SELECT
    json_get_str(data, 'case_id') as case_id,
    LAG(json_get_str(data, 'activity')) OVER w as prev_activity,
    json_get_str(data, 'activity') as current_activity,
    LEAD(json_get_str(data, 'activity')) OVER w as next_activity
FROM $table
WINDOW w AS (PARTITION BY json_get_str(data, 'case_id') ORDER BY created_at)

-- Event position in process
SELECT
    json_get_str(data, 'case_id') as case_id,
    json_get_str(data, 'activity') as activity,
    ROW_NUMBER() OVER (
        PARTITION BY json_get_str(data, 'case_id')
        ORDER BY created_at
    ) as step_number,
    COUNT(*) OVER (
        PARTITION BY json_get_str(data, 'case_id')
    ) as total_steps
FROM $table
```

### Process Metrics

```sql
-- Activity frequency
SELECT
    json_get_str(data, 'activity') as activity,
    COUNT(*) as occurrences,
    COUNT(DISTINCT json_get_str(data, 'case_id')) as unique_cases
FROM $table
GROUP BY json_get_str(data, 'activity')
ORDER BY occurrences DESC

-- First and last activities per case
SELECT DISTINCT
    json_get_str(data, 'case_id') as case_id,
    FIRST_VALUE(json_get_str(data, 'activity')) OVER w as start_activity,
    LAST_VALUE(json_get_str(data, 'activity')) OVER w as end_activity,
    MIN(created_at) OVER w as start_time,
    MAX(created_at) OVER w as end_time
FROM $table
WINDOW w AS (
    PARTITION BY json_get_str(data, 'case_id')
    ORDER BY created_at
    ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
)
```

### Transition Analysis

```sql
-- Activity transition matrix
WITH events AS (
    SELECT
        json_get_str(data, 'case_id') as case_id,
        json_get_str(data, 'activity') as activity,
        LEAD(json_get_str(data, 'activity')) OVER (
            PARTITION BY json_get_str(data, 'case_id')
            ORDER BY created_at
        ) as next_activity
    FROM $table
)
SELECT
    activity as from_activity,
    next_activity as to_activity,
    COUNT(*) as transitions
FROM events
WHERE next_activity IS NOT NULL
GROUP BY activity, next_activity
ORDER BY transitions DESC
```

### Bottleneck Detection

```sql
-- Time between events
WITH event_times AS (
    SELECT
        json_get_str(data, 'case_id') as case_id,
        json_get_str(data, 'activity') as activity,
        created_at,
        LAG(created_at) OVER (
            PARTITION BY json_get_str(data, 'case_id')
            ORDER BY created_at
        ) as prev_time
    FROM $table
)
SELECT
    activity,
    COUNT(*) as occurrences
FROM event_times
WHERE prev_time IS NOT NULL
GROUP BY activity
ORDER BY occurrences DESC

-- Cases with potential rework (more events than unique activities)
SELECT
    json_get_str(data, 'case_id') as case_id,
    COUNT(*) as total_events,
    COUNT(DISTINCT json_get_str(data, 'activity')) as unique_activities
FROM $table
GROUP BY json_get_str(data, 'case_id')
HAVING COUNT(*) > COUNT(DISTINCT json_get_str(data, 'activity'))
ORDER BY total_events DESC
```

### Process Variants

```sql
-- Discover unique process paths
WITH paths AS (
    SELECT
        json_get_str(data, 'case_id') as case_id,
        STRING_AGG(json_get_str(data, 'activity'), ' -> ' ORDER BY created_at) as process_path
    FROM $table
    GROUP BY json_get_str(data, 'case_id')
)
SELECT
    process_path,
    COUNT(*) as case_count
FROM paths
GROUP BY process_path
ORDER BY case_count DESC
LIMIT 20
```

### Case Duration Analysis

```sql
-- Case duration statistics
WITH case_times AS (
    SELECT
        json_get_str(data, 'case_id') as case_id,
        MIN(created_at) as start_time,
        MAX(created_at) as end_time
    FROM $table
    GROUP BY json_get_str(data, 'case_id')
)
SELECT
    COUNT(*) as total_cases,
    MIN(end_time) as earliest_completion,
    MAX(end_time) as latest_completion
FROM case_times
```
