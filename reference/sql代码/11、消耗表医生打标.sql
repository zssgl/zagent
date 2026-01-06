-- Step 1: Create empty table first
DROP TABLE IF EXISTS report_consumption_doctor;
CREATE TABLE report_consumption_doctor (
    病历编号 VARCHAR(255),
    顾客姓名 VARCHAR(255),
    消耗时间 DATETIME,
    `项目ID` VARCHAR(255),
    项目商品 VARCHAR(255),
    消耗诊所 VARCHAR(255),
    消耗数量 INT,
     `订单来源` INT,
      `是否是内购订单` INT,
    医生业绩 INT,
    doc_count INT,
    doctor_name VARCHAR(255)
);

-- Step 2: Insert data
INSERT INTO report_consumption_doctor
SELECT * FROM (
    SELECT 病历编号, 顾客姓名,消耗时间,`项目ID`,项目商品,消耗诊所, 消耗数量,`订单来源`,`是否是内购订单`,医生业绩, doc_count,
        SUBSTRING_INDEX(doctors, ',', 1) as doctor_name
    FROM report_consumption_employee
    WHERE doc_count IN (1,2)
    UNION ALL 
    SELECT 病历编号, 顾客姓名,消耗时间,`项目ID`,项目商品,消耗诊所, 消耗数量,`订单来源`,`是否是内购订单`,医生业绩, doc_count,
        SUBSTRING_INDEX(SUBSTRING_INDEX(doctors, ',', 2), ',', -1) as doctor_name
    FROM report_consumption_employee
    WHERE doc_count = 2
) t 
WHERE doctor_name IS NOT NULL
ORDER BY 消耗时间 DESC;