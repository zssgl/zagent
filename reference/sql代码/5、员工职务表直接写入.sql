-- Step 1: 删除已有表
DROP TABLE IF EXISTS bi.report_employee;

-- Step 2: 创建新表
CREATE TABLE bi.report_employee (
    `员工编号` VARCHAR(255),
    `员工姓名` VARCHAR(255),
    `是否在职` VARCHAR(255),
    `职务` VARCHAR(255),
    `具体职务` VARCHAR(255)
    
);

-- Step 3: 插入数据到表中（排除空值）
INSERT INTO bi.report_employee(
    `员工编号`, `员工姓名`,`是否在职`,`职务`,`具体职务`
)



SELECT 
    e.EmpNo AS 员工编号,
    e.EmpName AS 员工姓名,
    CASE 
        WHEN e.Status = '0' THEN '在职'
        ELSE '离职'
    END AS 是否在职,
        CASE 
        WHEN c.DisplayName IN ('高级护士', '中级护士') THEN '护士'
        ELSE c.DisplayName
    END AS 职务,
    c.DisplayName AS 具体职务

FROM 
    employees e
LEFT JOIN 
    customdictionary c 
ON 
    e.JobTitle_ID = c.ID
GROUP BY 
    e.EmpNo, e.EmpName, e.Status, c.DisplayName;