
DROP TABLE IF EXISTS report_bill_employee;
CREATE TABLE report_bill_employee (
    `开单门店` VARCHAR(255),
    `病历编号` VARCHAR(255),
    `顾客姓名` VARCHAR(255),
    `性别` VARCHAR(10),
    `客户状态` VARCHAR(50),
    `咨询师` VARCHAR(255),
    `项目类型` VARCHAR(255),
    `项目分类` VARCHAR(255),
    `项目ID` VARCHAR(255),
    `开单项目` VARCHAR(255),
    `客户来源` VARCHAR(255),
    `规格` VARCHAR(255),
    `数量` INT,
    `单位` VARCHAR(50),
    `支付方式` VARCHAR(255),
    `实收金额` DECIMAL(15, 2),
    `开单人` VARCHAR(255),
    `业绩人` VARCHAR(255),
    `权益人` VARCHAR(255),
    `付款时间` DATETIME,
    `最早开单时间` DATETIME,
    `最后消耗时间` DATETIME,
    `开单时间` DATETIME,
    `备注` TEXT,
    `实质消费` DECIMAL(15, 2),
    `促销活动` VARCHAR(255),
    `组合项目` VARCHAR(255),
    `是否是网络订单` BOOLEAN,
    `是否是促销项目` BOOLEAN,
    `客户类型` INT,
    `开单提点类型` VARCHAR(255),
    `订单来源` INT,
    
    `是否是内购订单` INT,
		`doc_count` INT,
   `nurse_count` INT,
   `cons_count` INT,
   `doctors` VARCHAR(255),
   `nurses` VARCHAR(255),
   `consultants` VARCHAR(255),
   `医生业绩` INT,
   `护士咨询业绩` INT
);

-- Step 3: Insert data into the table
INSERT INTO report_bill_employee
SELECT 
 t.*,
 CASE 
   WHEN doc_count > 0 THEN ROUND(t.实质消费 / doc_count, 2)
   ELSE 0 
 END as 医生业绩,
 CASE 
   WHEN (nurse_count + cons_count) > 0 THEN ROUND(t.实质消费 / (nurse_count + cons_count), 2)
   ELSE 0
 END as 护士咨询业绩
FROM (
 SELECT 
   base.*,
   COUNT(DISTINCT CASE WHEN e.具体职务 = '医生' THEN e.员工姓名 END) as doc_count,
   COUNT(DISTINCT CASE WHEN e.具体职务 IN ('中级护士','高级护士') THEN e.员工姓名 END) as nurse_count,
   COUNT(DISTINCT CASE WHEN e.具体职务 IN ('咨询师','大客户主管') THEN e.员工姓名 END) as cons_count,
   GROUP_CONCAT(DISTINCT CASE WHEN e.具体职务 = '医生' THEN e.员工姓名 END) as doctors,
   GROUP_CONCAT(DISTINCT CASE WHEN e.具体职务 IN ('中级护士','高级护士') THEN e.员工姓名 END) as nurses,
   GROUP_CONCAT(DISTINCT CASE WHEN e.具体职务 IN ('咨询师','大客户主管') THEN e.员工姓名 END) as consultants
 FROM report_bill_detail base
 CROSS JOIN (
   SELECT 1 AS n UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4
 ) n
 LEFT JOIN report_employee e ON 
   TRIM(SUBSTRING_INDEX(SUBSTRING_INDEX(base.业绩人, ',', n.n), ',', -1)) = e.员工姓名 
 WHERE n.n <= 1 + LENGTH(base.业绩人) - LENGTH(REPLACE(base.业绩人, ',', ''))
 GROUP BY base.开单门店, base.病历编号, base.顾客姓名, base.性别, base.客户状态, base.咨询师,
         base.项目类型, base.项目分类, base.`项目ID`, base.开单项目, base.客户来源, base.规格, base.数量,
         base.单位, base.支付方式, base.实收金额, base.开单人, base.业绩人, base.权益人,
         base.付款时间, base.最早开单时间, base.最后消耗时间, base.开单时间, base.备注,
         base.实质消费, base.促销活动, base.组合项目, base.是否是网络订单, base.是否是促销项目,
         base.客户类型, base.开单提点类型
) t
ORDER BY 开单时间 DESC;