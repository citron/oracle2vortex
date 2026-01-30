-- Example SQL query for testing oracle2vortex
-- This query fetches sample employee data from the HR schema
SELECT 
    employee_id,
    first_name,
    last_name,
    email,
    phone_number,
    hire_date,
    job_id,
    salary,
    commission_pct,
    manager_id,
    department_id
FROM employees
WHERE rownum <= 100;
